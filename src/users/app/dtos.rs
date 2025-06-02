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
