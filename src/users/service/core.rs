use super::errors::Error;
use super::idos::{User, UserUpdate};
use uuid::Uuid;

use crate::users::repo;

// pub trait Repo {
//     // fn get_user(&self, user_id: &str) -> Result<idos::User, String>;
//     async fn create_user(&self, user: idos::User) -> Result<idos::User, String>;
//     // fn update_user(&self, user_id: &str, user: idos::User) -> Result<idos::User, String>;
//     // fn delete_user(&self, user_id: &str) -> Result<(), String>;
//     // fn list_users(&self) -> Result<Vec<idos::User>, String>;
// }

#[derive(Clone)]
pub struct Service {
    repo: repo::dynamodb::Repo,
}

impl Service {
    pub fn new(repo: repo::dynamodb::Repo) -> Self {
        Self { repo }
    }

    pub async fn create_user(&self, user: User) -> Result<User, Error> {
        if user.id.is_nil() {
            tracing::error!("missing uuid");
            return Err(Error::Validation("user id must be populated".to_string()));
        }

        match self.repo.create_user(&user).await {
            Ok(_) => Ok(user),
            Err(e) => Err(Error::from_repo_error(e)),
        }
    }

    pub async fn get_user(&self, id: Uuid) -> Result<User, Error> {
        if id.is_nil() {
            tracing::error!("missing uuid");
            return Err(Error::Validation("user id must be populated".to_string()));
        }

        match self.repo.get_user(id).await {
            Ok(user) => Ok(user),
            Err(e) => Err(Error::from_repo_error(e)),
        }
    }

    pub async fn update_user(&self, id: Uuid, update: UserUpdate) -> Result<User, Error> {
        if id.is_nil() {
            tracing::error!("missing uuid");
            return Err(Error::Validation("user id must be populated".to_string()));
        }

        if update.first_name.is_none()
            && update.last_name.is_none()
            && update.email.is_none()
            && update.dob.is_none()
        {
            return Err(Error::MissingParameters(
                "at least one field must be updated".to_string(),
            ));
        }

        // fetch the existing user to update
        let mut user = match self.repo.get_user(id).await {
            Ok(user) => user,
            Err(e) => return Err(Error::from_repo_error(e)),
        };

        user.update(update);

        match self.repo.update_user(&user).await {
            Ok(_) => Ok(user),
            Err(e) => Err(Error::from_repo_error(e)),
        }
    }
}
