use crate::users::service::idos;
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserCreationReq {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub dob: String,
}

#[derive(Deserialize)]
pub struct UserUpdateReq {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub dob: Option<String>,
}

impl UserUpdateReq {
    pub fn into_update(self) -> Result<idos::UserUpdate, String> {
        let dob = match self.dob {
            Some(date_str) => match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    return Err("Invalid date format for dob, expected YYYY-MM-DD".to_string());
                }
            },
            None => None,
        };

        Ok(idos::UserUpdate {
            first_name: self.first_name,
            last_name: self.last_name,
            email: self.email,
            dob,
        })
    }
}
