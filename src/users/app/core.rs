use super::dtos::{QueryUser, ReqUserCreation, ReqUserUpdate};
use super::errors::from_service_error;
use super::state::State;
use crate::users::service;
use crate::users::service::errors::Error as ServiceError;

use std::sync::Arc;

use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use chrono::NaiveDate;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait Service: Send + Sync + 'static {
    async fn create_user(
        &self,
        user: service::idos::User,
    ) -> Result<service::idos::User, ServiceError>;

    async fn get_user(&self, id: Uuid) -> Result<service::idos::User, ServiceError>;

    async fn get_user_by_email(&self, email: &str) -> Result<service::idos::User, ServiceError>;

    async fn update_user(
        &self,
        id: Uuid,
        update: service::idos::UserUpdate,
    ) -> Result<service::idos::User, ServiceError>;

    async fn delete_user(&self, id: Uuid) -> Result<(), ServiceError>;
}

#[derive(Clone)]
pub struct App {
    scope: String,
    state: State,
    service: Arc<dyn Service>,
}

impl App {
    pub fn new(scope: String, service: Arc<dyn Service>) -> Self {
        Self {
            scope,
            state: State::new(),
            service: service,
        }
    }

    pub fn configure(&self) -> impl FnOnce(&mut web::ServiceConfig) + Clone {
        let scope = self.scope.clone();
        let state = self.state.clone();
        let service: Arc<dyn Service + 'static> = self.service.clone();

        move |cfg: &mut web::ServiceConfig| {
            cfg.service(
                web::scope(&scope)
                    .app_data(web::Data::new(state))
                    .app_data::<actix_web::web::Data<Arc<dyn Service>>>(web::Data::new(service))
                    .service(create_user)
                    .service(get_user_by_id)
                    .service(get_user_by_email)
                    .service(update_user)
                    .service(delete_user),
            );
        }
    }
}

#[post("")]
async fn create_user(
    service: web::Data<Arc<dyn Service>>,
    user: web::Json<ReqUserCreation>,
) -> impl Responder {
    let new_user_req: ReqUserCreation = user.into_inner();

    let parsed_dob = NaiveDate::parse_from_str(&new_user_req.dob, "%Y-%m-%d");
    match parsed_dob {
        Err(e) => {
            println!("Failed to parse date: {}", e);
            // send back 400
            return HttpResponse::BadRequest().body("could not parse dob");
        }
        Ok(_) => println!("good date"),
    }

    let new_user = service::idos::User::new(
        new_user_req.first_name,
        new_user_req.last_name,
        new_user_req.email,
        parsed_dob.unwrap(),
    );

    // pass new user to service.create_user
    let created_user = match service.create_user(new_user.clone()).await {
        Ok(user) => user,
        Err(e) => {
            println!("Failed to create user: {}", e);
            return from_service_error(e);
        }
    };

    HttpResponse::Created()
        .content_type("application/json")
        .body(serde_json::to_string(&created_user).unwrap_or_else(|_| "{}".to_string()))
}

#[get("/{id}")]
async fn get_user_by_id(
    service: web::Data<Arc<dyn Service>>,
    user_id: web::Path<String>,
) -> impl Responder {
    // this next step can fail if the string provided is not a valid uuid
    let user_uuid_str: &String = &user_id.into_inner();
    let user_uuid = match uuid::Uuid::parse_str(user_uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid UUID format"),
    };

    match service.get_user(user_uuid).await {
        Ok(user) => HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&user).unwrap_or_else(|_| "{}".to_string())),
        Err(e) => from_service_error(e),
    }
}

#[get("")]
async fn get_user_by_email(
    service: web::Data<Arc<dyn Service>>,
    email: web::Query<QueryUser>,
) -> impl Responder {
    let email_str_opt = email.into_inner().email;

    let email_str = match email_str_opt {
        Some(email) => email,
        None => return HttpResponse::BadRequest().body("Email query parameter is required"),
    };

    match service.get_user_by_email(&email_str).await {
        Ok(user) => HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&user).unwrap_or_else(|_| "{}".to_string())),
        Err(e) => from_service_error(e),
    }
}

#[put("/{id}")]
async fn update_user(
    service: web::Data<Arc<dyn Service>>,
    user_id: web::Path<String>,
    user: web::Json<ReqUserUpdate>,
) -> impl Responder {
    let user_uuid_str = &user_id.into_inner();
    let user_uuid = match uuid::Uuid::parse_str(user_uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid UUID format"),
    };

    let user_update = match user.into_inner().into_update() {
        Ok(update) => update,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    match service.update_user(user_uuid, user_update).await {
        Ok(updated_user) => HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&updated_user).unwrap_or_else(|_| "{}".to_string())),
        Err(e) => from_service_error(e),
    }
}

#[delete("/{id}")]
async fn delete_user(
    service: web::Data<Arc<dyn Service>>,
    user_id: web::Path<String>,
) -> impl Responder {
    let user_uuid_str = &user_id.into_inner();
    let user_uuid = match uuid::Uuid::parse_str(user_uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid UUID format"),
    };

    match service.delete_user(user_uuid).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => from_service_error(e),
    }
}
