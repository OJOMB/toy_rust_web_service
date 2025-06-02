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
}
