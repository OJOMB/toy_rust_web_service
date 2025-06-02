use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub dob: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(first_name: String, last_name: String, email: String, dob: NaiveDate) -> Self {
        let now = Utc::now();
        User {
            id: Uuid::new_v4(),
            first_name,
            last_name,
            email,
            dob,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_dummy() -> Self {
        User {
            id: Uuid::from_u128(0),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "jd@example.com".to_string(),
            dob: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap(),
        }
    }

    pub fn update(&mut self, update: UserUpdate) {
        if let Some(first_name) = update.first_name {
            self.first_name = first_name;
        }

        if let Some(last_name) = update.last_name {
            self.last_name = last_name;
        }

        if let Some(email) = update.email {
            self.email = email;
        }

        if let Some(dob) = update.dob {
            self.dob = dob;
        }

        self.updated_at = Utc::now();
    }
}

pub struct UserUpdate {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub dob: Option<NaiveDate>,
}
