use std::collections::HashMap;

use super::errors::Error;
use crate::users::service;
use crate::users::service::idos;

use aws_sdk_dynamodb::operation::{
    delete_item::DeleteItemError, put_item::PutItemError, update_item::UpdateItemError,
};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_smithy_runtime_api::client::result::SdkError;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::{self, Uuid};

#[derive(Clone)]
pub struct Repo {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
    lookup_table_name: String,
}

impl Repo {
    pub fn new(
        client: aws_sdk_dynamodb::Client,
        table_name: String,
        lookup_table_name: String,
    ) -> Self {
        Self {
            client,
            table_name,
            lookup_table_name,
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
                return Err(Error::Internal("unexpected repo error".to_string()));
            }
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<idos::User, Error> {
        // email is a unique identifier in our system, so we can use it to look up the user
        let request = self
            .client
            .get_item()
            .table_name(self.lookup_table_name.clone())
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
                                    Err(Error::Internal("unexpected repo error".to_string()))
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
                return Err(Error::Internal("unexpected repo error".to_string()));
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
            .table_name(self.lookup_table_name.clone())
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
                        return Err(Error::Internal("unexpected repo error".to_string()));
                    }
                },
                _ => {
                    tracing::error!("Unexpected error occurred: {:?}", e);
                    return Err(Error::Internal("unexpected repo error".to_string()));
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
                    .table_name(self.lookup_table_name.clone())
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

                return Err(Error::Internal("unexpected repo error".to_string()));
            }
        }
    }

    async fn update_user(&self, user: &idos::User) -> Result<(), Error> {
        let request = self
            .client
            .update_item()
            .table_name(self.table_name.clone())
            .key("id".to_string(), AttributeValue::S(user.id.to_string()))
            .update_expression(
                "SET first_name = :first_name, last_name = :last_name, email = :email, dob = :dob, updated_at = :updated_at",
            )
            // Ensure the item exists before updating to prevent silent creation
            .condition_expression("attribute_exists(id)")
            .expression_attribute_values(":first_name", AttributeValue::S(user.first_name.clone()))
            .expression_attribute_values(":last_name", AttributeValue::S(user.last_name.clone()))
            .expression_attribute_values(":email", AttributeValue::S(user.email.clone()))
            .expression_attribute_values(":dob", AttributeValue::S(user.dob.to_string()))
            .expression_attribute_values(
                ":updated_at",
                AttributeValue::S(user.updated_at.to_rfc3339()),
            );

        let resp = request.send().await;
        match resp {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to update user: {:?}", e);
                match e {
                    SdkError::ServiceError(service_err) => match service_err.err() {
                        UpdateItemError::ConditionalCheckFailedException(_) => Err(Error::NotFound),
                        _ => Err(Error::Internal("unexpected repo error".to_string())),
                    },
                    SdkError::TimeoutError(_) => {
                        tracing::error!("Timeout error while updating user");
                        Err(Error::Internal("timeout error".to_string()))
                    }
                    _ => Err(Error::Internal("unexpected repo error".to_string())),
                }
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
                        _ => Err(Error::Internal("unexpected repo error".to_string())),
                    },
                    SdkError::TimeoutError(_) => {
                        tracing::error!("Timeout error while updating user");
                        Err(Error::Internal("timeout error".to_string()))
                    }
                    _ => Err(Error::Internal("unexpected repo error".to_string())),
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
