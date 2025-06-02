use std::collections::HashMap;

use super::errors::Error;
use crate::users::service::idos;

use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::{self, Uuid};

#[derive(Clone)]
pub struct Repo {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
}

impl Repo {
    pub fn new(client: aws_sdk_dynamodb::Client, table_name: String) -> Self {
        Self { client, table_name }
    }
}

impl Repo {
    pub async fn get_user(&self, id: Uuid) -> Result<idos::User, Error> {
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

    pub async fn create_user(&self, user: &idos::User) -> Result<(), Error> {
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
                return Err(Error::Internal("unexpected repo error".to_string()));
            }
        }
    }

    // pub async fn update_user(&self, user_id: &str, user: &UserUpdateReq) -> Result<(), String> {
    //     // Implementation for updating a user in DynamoDB
    //     Ok(())
    // }

    // pub async fn delete_user(&self, user_id: &str) -> Result<(), String> {
    //     // Implementation for deleting a user from DynamoDB
    //     Ok(())
    // }
}

fn user_from_attrs(attrs: &HashMap<String, AttributeValue>) -> Result<idos::User, Error> {
    let id = get_string(attrs, "id").and_then(|val| {
        Uuid::parse_str(&val).map_err(|_| Error::MalformedRecord("invalid uuid".to_string()))
    })?;

    let first_name = get_optional_string(attrs, "first_name")?;
    let last_name = get_optional_string(attrs, "last_name")?;
    let email = get_string(attrs, "email")?;
    let dob = get_string(attrs, "dob").and_then(|val| {
        NaiveDate::parse_from_str(&val, "%Y-%m-%d")
            .map_err(|_| Error::MalformedRecord("invalid dob format".to_string()))
    })?;

    let created_at = get_string(attrs, "created_at").and_then(|val| {
        DateTime::parse_from_rfc3339(&val)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| Error::MalformedRecord("invalid created_at format".to_string()))
    })?;

    let updated_at = get_string(attrs, "updated_at").and_then(|val| {
        DateTime::parse_from_rfc3339(&val)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| Error::MalformedRecord("invalid updated_at format".to_string()))
    })?;

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

fn get_string(attrs: &HashMap<String, AttributeValue>, key: &str) -> Result<String, Error> {
    match attrs.get(key) {
        Some(AttributeValue::S(val)) => Ok(val.clone()),
        Some(_) => Err(Error::MalformedRecord(format!(
            "incorrect type for {}",
            key
        ))),
        None => Err(Error::MalformedRecord(format!("{} missing", key))),
    }
}

fn get_optional_string(
    attrs: &HashMap<String, AttributeValue>,
    key: &str,
) -> Result<String, Error> {
    Ok(match attrs.get(key) {
        Some(AttributeValue::S(val)) => val.clone(),
        Some(_) => {
            return Err(Error::MalformedRecord(format!(
                "incorrect type for {}",
                key
            )));
        }
        None => "".to_string(),
    })
}
