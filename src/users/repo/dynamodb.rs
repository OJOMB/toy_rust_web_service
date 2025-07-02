use std::collections::HashMap;

use super::errors::Error;
use crate::users::service;
use crate::users::service::idos;

use aws_sdk_dynamodb::operation::{delete_item::DeleteItemError, put_item::PutItemError};

use aws_sdk_dynamodb::{
    Client,
    error::SdkError,
    operation::transact_write_items::TransactWriteItemsError::TransactionCanceledException,
    types::{AttributeValue, Delete, Put, TransactWriteItem},
};
use chrono::{DateTime, NaiveDate, Utc};
use uuid::{self, Uuid};

const OPERATION_LABELS: [&str; 3] = ["delete_old_email_with_check", "put_new_email", "put_user"];

#[derive(Clone)]
pub struct Repo {
    client: Client,
    table_name: String,
    email_lookup_table_name: String,
}

impl Repo {
    pub fn new(client: Client, table_name: String, email_lookup_table_name: String) -> Self {
        Self {
            client,
            table_name,
            email_lookup_table_name,
        }
    }
}

#[async_trait::async_trait]
impl service::core::Repo for Repo {
    async fn get_user(&self, id: Uuid) -> Result<idos::User, Error> {
        let request = self
            .client
            .get_item()
            .table_name(self.table_name.clone())
            .key("id".to_string(), AttributeValue::S(id.to_string()));

        let resp = request.send().await;
        match resp {
            Ok(output) => match output.item() {
                Some(attrs) => user_from_attrs(attrs),
                None => Err(Error::NotFound),
            },
            Err(e) => {
                println!("{:?}", e);
                return Err(Error::Internal);
            }
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<idos::User, Error> {
        // email is a unique identifier in our system, so we can use it to look up the user
        let request = self
            .client
            .get_item()
            .table_name(self.email_lookup_table_name.clone())
            .key("email".to_string(), AttributeValue::S(email.to_string()));

        let resp = request.send().await;
        match resp {
            Ok(output) => match output.item() {
                Some(attrs) => {
                    // we have the lookup entry, now we can get the user by id
                    let user_id = get_string(attrs, "id")?;

                    let uuid = Uuid::parse_str(&user_id);
                    match uuid {
                        Ok(uuid) => {
                            tracing::info!("Found user with email: {}", email);
                            let user = self.get_user(uuid).await;
                            match user {
                                Ok(user) => Ok(user),
                                Err(Error::NotFound) => {
                                    tracing::error!(
                                        "User record not found for email that exists in lookup table: {}",
                                        email
                                    );
                                    Err(Error::NotFound)
                                }
                                Err(e) => {
                                    tracing::error!("Error fetching user: {}", e);
                                    Err(Error::Internal)
                                }
                            }
                        }
                        Err(_) => {
                            tracing::error!(
                                "Invalid UUID format for user ID in email lookup table: {}",
                                user_id
                            );
                            return Err(Error::Validation("invalid uuid format".to_string()));
                        }
                    }
                }
                None => Err(Error::NotFound),
            },
            Err(e) => {
                println!("{:?}", e);
                return Err(Error::Internal);
            }
        }
    }

    async fn create_user(&self, user: &idos::User) -> Result<(), Error> {
        // first we attempt to create an entry in the lookup table under the condition that the given email does not already exist
        // this serves as both a check to ensure that we do not have duplicate emails in the system and as a way to create a lookup entry for the user
        // if the email already exists, we will get a ConditionalCheckFailedException
        let lookup_request = self
            .client
            .put_item()
            .table_name(self.email_lookup_table_name.clone())
            .item("id".to_string(), AttributeValue::S(user.id.to_string()))
            .item("email".to_string(), AttributeValue::S(user.email.clone()))
            .condition_expression("attribute_not_exists(email)");

        let lookup_resp = lookup_request.send().await;
        match lookup_resp {
            Ok(_) => {
                tracing::info!(
                    "Lookup entry created successfully for email: {}",
                    user.email
                );
            }
            Err(ref e) => match e {
                SdkError::ServiceError(service_err) => match service_err.err() {
                    PutItemError::ConditionalCheckFailedException(_) => {
                        tracing::info!(
                            "creation of email lookup record failed as a user with the given address already exists"
                        );
                        return Err(Error::EmailAddressAlreadyInUse(
                            "a user with the given email already exists".to_string(),
                        ));
                    }
                    _ => {
                        tracing::error!("Unexpected error occurred: {:?}", e);
                        return Err(Error::Internal);
                    }
                },
                _ => {
                    tracing::error!("Unexpected error occurred: {:?}", e);
                    return Err(Error::Internal);
                }
            },
        }

        let request = self
            .client
            .put_item()
            .table_name(self.table_name.clone())
            .item("id".to_string(), AttributeValue::S(user.id.to_string()))
            .item(
                "first_name".to_string(),
                AttributeValue::S(user.first_name.clone()),
            )
            .item(
                "last_name".to_string(),
                AttributeValue::S(user.last_name.clone()),
            )
            .item("email".to_string(), AttributeValue::S(user.email.clone()))
            .item("dob".to_string(), AttributeValue::S(user.dob.to_string()))
            .item(
                "created_at".to_string(),
                AttributeValue::S(user.created_at.to_rfc3339()),
            )
            .item(
                "updated_at".to_string(),
                AttributeValue::S(user.updated_at.to_rfc3339()),
            );

        println!("Executing request [{request:?}] to add item...");

        let resp = request.send().await;
        match resp {
            Ok(_) => {
                println!("User created successfully");
                Ok(())
            }
            Err(e) => {
                println!("Failed to create user: {:?}", e);

                // rollback the lookup entry if the user creation fails
                let rollback_request = self
                    .client
                    .delete_item()
                    .table_name(self.email_lookup_table_name.clone())
                    .key("id".to_string(), AttributeValue::S(user.id.to_string()));

                let rollback_resp = rollback_request.send().await;
                if let Err(rollback_err) = rollback_resp {
                    tracing::error!(
                        "Failed to rollback lookup entry for user {}: {:?}",
                        user.id,
                        rollback_err
                    );
                } else {
                    tracing::info!(
                        "Rolled back lookup entry for user {} after failed creation",
                        user.id
                    );
                }

                return Err(Error::Internal);
            }
        }
    }

    async fn update_user(
        &self,
        user: &idos::User,
        old_user_email: Option<String>,
    ) -> Result<(), Error> {
        let mut tx_write_items: Vec<TransactWriteItem> = vec![];

        // old_user_email is None if the email has not changed
        if let Some(old_email) = old_user_email.clone() {
            // outside the tx we perform a pre-write lookup
            let old_email_lookup_record = self
                .client
                .get_item()
                .table_name(self.email_lookup_table_name.clone())
                .key("email", AttributeValue::S(old_email.to_string()))
                .send()
                .await;

            match old_email_lookup_record {
                Ok(get_item_output) => {
                    let attrs = match get_item_output.item() {
                        Some(attrs) => attrs,
                        None => {
                            tracing::error!(
                                "Old email lookup record not found for email: {}, user ID: {}",
                                old_email,
                                user.id
                            );

                            return Err(Error::Internal);
                        }
                    };

                    let id = get_string(attrs, "id")?;
                    if id != user.id.to_string() {
                        return Err(Error::EmailAddressAlreadyInUse(
                            "a user with the provided email already exists".to_string(),
                        ));
                    }
                }
                Err(_) => {
                    tracing::error!(
                        "Failed to retrieve old email lookup record for email: {}, user ID: {}",
                        old_email,
                        user.id
                    );
                    return Err(Error::Internal);
                }
            }

            let delete_old_email_with_check: TransactWriteItem = TransactWriteItem::builder()
                .delete(
                    Delete::builder()
                        .table_name(self.email_lookup_table_name.clone())
                        .key("email", AttributeValue::S(old_email.to_string()))
                        .expression_attribute_values(":id", AttributeValue::S(user.id.to_string()))
                        .build()
                        .unwrap(),
                )
                .build();

            tx_write_items.push(delete_old_email_with_check);

            let put_new_email = TransactWriteItem::builder()
                .put(
                    Put::builder()
                        .table_name(self.email_lookup_table_name.clone())
                        .item("email", AttributeValue::S(user.email.to_string()))
                        .item("id", AttributeValue::S(user.id.to_string()))
                        .condition_expression("attribute_not_exists(email)")
                        .build()
                        .unwrap(),
                )
                .build();

            tx_write_items.push(put_new_email);
        }

        let put_user = TransactWriteItem::builder()
            .put(
                Put::builder()
                    .table_name(self.table_name.clone())
                    .item("id", AttributeValue::S(user.id.to_string()))
                    .item("email", AttributeValue::S(user.email.to_string()))
                    .item("first_name", AttributeValue::S(user.first_name.clone()))
                    .item("last_name", AttributeValue::S(user.last_name.clone()))
                    .item("dob", AttributeValue::S(user.dob.to_string()))
                    .item(
                        "updated_at",
                        AttributeValue::S(user.updated_at.to_rfc3339()),
                    )
                    .condition_expression("attribute_exists(id)") // Ensure the item exists before updating to prevent silent creation
                    .build()
                    .unwrap(),
            )
            .build();

        tx_write_items.push(put_user);

        let result = self
            .client
            .transact_write_items()
            .set_transact_items(Some(tx_write_items))
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(SdkError::ServiceError(svc_err)) => {
                tracing::error!("Blud Transaction failed");
                match svc_err.err() {
                    TransactionCanceledException(e) => {
                        tracing::error!("Transaction was cancelled: {:?}", svc_err);

                        for (i, maybe_reason) in e.cancellation_reasons().iter().enumerate() {
                            if let Some(reason) = maybe_reason.code.clone() {
                                let op = OPERATION_LABELS.get(i).unwrap_or(&"unknown op");
                                let msg = maybe_reason.message().unwrap_or("no message");
                                tracing::error!(
                                    "Step {} - op: {} failed: [{}] {}",
                                    i + 1,
                                    op,
                                    reason,
                                    msg
                                );

                                let _cond_check_failed = "ConditionalCheckFailed";
                                match (*op, reason.as_str()) {
                                    ("put_new_email", _cond_check_failed) => {
                                        return Err(Error::EmailAddressAlreadyInUse(
                                            "a user with the provided email already exists"
                                                .to_string(),
                                        ));
                                    }
                                    ("put_user", _cond_check_failed) => {
                                        return Err(Error::NotFound);
                                    }
                                    (op, reason) => {
                                        tracing::error!(
                                            "Unhandled operation {} with reason: {}",
                                            op,
                                            reason
                                        );
                                        return Err(Error::Internal);
                                    }
                                }
                            }
                        }

                        return Err(Error::Internal);
                    }
                    _ => {
                        tracing::error!("Unexpected error occurred: {:?}", svc_err);
                        return Err(Error::Internal);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Non-service error: {:?}", e);
                return Err(Error::Internal);
            }
        }
    }

    async fn delete_user(&self, id: Uuid) -> Result<(), Error> {
        let request = self
            .client
            .delete_item()
            .table_name(self.table_name.clone())
            .key("id".to_string(), AttributeValue::S(id.to_string()));

        let resp = request.send().await;
        match resp {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Failed to delete user: {:?}", e);
                match e {
                    SdkError::ServiceError(service_err) => match service_err.err() {
                        DeleteItemError::ConditionalCheckFailedException(_) => Err(Error::NotFound),
                        _ => Err(Error::Internal),
                    },
                    SdkError::TimeoutError(_) => {
                        tracing::error!("Timeout error while updating user");
                        Err(Error::Internal)
                    }
                    _ => Err(Error::Internal),
                }
            }
        }
    }
}

fn user_from_attrs(attrs: &HashMap<String, AttributeValue>) -> Result<idos::User, Error> {
    let id = get_string(attrs, "id").and_then(|val| {
        Uuid::parse_str(&val).map_err(|_| Error::Validation("invalid uuid".to_string()))
    })?;

    let first_name = get_optional_string(attrs, "first_name")?;
    let last_name = get_optional_string(attrs, "last_name")?;
    let email = get_string(attrs, "email")?;
    let dob = get_string(attrs, "dob").and_then(|val| {
        NaiveDate::parse_from_str(&val, "%Y-%m-%d")
            .map_err(|_| Error::MalformedResponse("invalid dob format".to_string()))
    })?;

    let created_at = get_datetime(attrs, "updated_at")?;
    let updated_at = get_datetime(attrs, "updated_at")?;

    Ok(idos::User {
        id,
        first_name,
        last_name,
        email,
        dob,
        created_at,
        updated_at,
    })
}

fn get_datetime(
    attrs: &HashMap<String, AttributeValue>,
    key: &str,
) -> Result<DateTime<Utc>, Error> {
    match attrs.get(key) {
        Some(AttributeValue::S(val)) => DateTime::parse_from_rfc3339(val)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| Error::MalformedResponse(format!("invalid {} format", key))),
        Some(_) => Err(Error::MalformedResponse(format!(
            "incorrect type for {}",
            key
        ))),
        None => Err(Error::Validation(format!("{} missing", key))),
    }
}

fn get_string(attrs: &HashMap<String, AttributeValue>, key: &str) -> Result<String, Error> {
    match attrs.get(key) {
        Some(AttributeValue::S(val)) => Ok(val.clone()),
        Some(_) => Err(Error::MalformedResponse(format!(
            "incorrect type for {}",
            key
        ))),
        None => Err(Error::Validation(format!("{} missing", key))),
    }
}

fn get_optional_string(
    attrs: &HashMap<String, AttributeValue>,
    key: &str,
) -> Result<String, Error> {
    Ok(match attrs.get(key) {
        Some(AttributeValue::S(val)) => val.clone(),
        Some(_) => {
            return Err(Error::MalformedResponse(format!(
                "incorrect type for {}",
                key
            )));
        }
        None => "".to_string(),
    })
}
