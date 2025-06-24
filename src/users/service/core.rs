use super::errors::Error;
use super::idos::{User, UserUpdate};
use crate::users::app::core::Service as AppService;
use crate::users::repo::errors::Error as RepoError;

use uuid::Uuid;

#[async_trait::async_trait]
pub trait Repo: Send + Sync + Clone + 'static {
    async fn create_user(&self, user: &User) -> Result<(), RepoError>;

    async fn get_user(&self, id: Uuid) -> Result<User, RepoError>;

    async fn get_user_by_email(&self, email: &str) -> Result<User, RepoError>;

    async fn update_user(&self, user: &User) -> Result<(), RepoError>;

    async fn delete_user(&self, id: Uuid) -> Result<(), RepoError>;
}

#[derive(Clone)]
pub struct Service<R: Repo> {
    repo: R,
}

impl<R: Repo> Service<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl<R: Repo> AppService for Service<R> {
    async fn create_user(&self, user: User) -> Result<User, Error> {
        if user.id.is_nil() {
            tracing::error!("missing uuid");
            return Err(Error::Validation("user id must be populated".to_string()));
        }

        match self.repo.create_user(&user).await {
            Ok(_) => Ok(user),
            Err(e) => Err(Error::from_repo_error(e)),
        }
    }

    async fn get_user(&self, id: Uuid) -> Result<User, Error> {
        if id.is_nil() {
            tracing::error!("missing uuid");
            return Err(Error::Validation("user id must be populated".to_string()));
        }

        match self.repo.get_user(id).await {
            Ok(user) => Ok(user),
            Err(e) => Err(Error::from_repo_error(e)),
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, Error> {
        if email.is_empty() {
            tracing::error!("missing email");
            return Err(Error::Validation("email must be populated".to_string()));
        }

        match self.repo.get_user_by_email(email).await {
            Ok(user) => Ok(user),
            Err(e) => Err(Error::from_repo_error(e)),
        }
    }

    async fn update_user(&self, id: Uuid, update: UserUpdate) -> Result<User, Error> {
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

    async fn delete_user(&self, id: Uuid) -> Result<(), Error> {
        if id.is_nil() {
            tracing::error!("missing uuid");
            return Err(Error::Validation("user id must be populated".to_string()));
        }

        match self.repo.delete_user(id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::from_repo_error(e)),
        }
    }
}
