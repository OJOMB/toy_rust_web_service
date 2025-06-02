use super::dtos::{UserCreationReq, UserUpdateReq};
use super::errors::from_service_error;
use super::state::State;
use crate::users::service;

use actix_web::{HttpResponse, Responder, get, post, put, web};
use chrono::NaiveDate;
use uuid;

#[derive(Clone)]
pub struct App {
    scope: String,
    state: State,
    service: service::core::Service,
}

impl App {
    pub fn new(scope: String, service: service::core::Service) -> Self {
        Self {
            scope,
            state: State::new(),
            service,
        }
    }

    pub fn configure(&self) -> impl FnOnce(&mut web::ServiceConfig) + Clone {
        let scope = self.scope.clone();
        let state = self.state.clone();
        let service = self.service.clone();

        move |cfg: &mut web::ServiceConfig| {
            cfg.service(
                web::scope(&scope)
                    .app_data(web::Data::new(state))
                    .app_data(web::Data::new(service))
                    .service(get_all_users)
                    .service(create_user)
                    .service(get_user_by_id)
                    .service(update_user),
            );
        }
    }
}

#[get("/")]
async fn get_all_users(state: web::Data<State>) -> impl Responder {
    // Lock the mutex to access the users hashmap
    let users_guard = state.users.lock().unwrap();
    let users: Vec<&service::idos::User> = users_guard.values().collect();
    if users.is_empty() {
        return HttpResponse::NotFound().body("No users found");
    }

    // Convert users to JSON and return
    let users_json = serde_json::to_string(&users).unwrap_or_else(|_| "[]".to_string());

    HttpResponse::Ok()
        .content_type("application/json")
        .body(users_json)
}

#[post("/")]
async fn create_user(
    service: web::Data<service::core::Service>,
    user: web::Json<UserCreationReq>,
) -> impl Responder {
    let new_user_req: UserCreationReq = user.into_inner();

    let parsed_dob = NaiveDate::parse_from_str(&new_user_req.dob, "%d-%m-%Y");
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
            return HttpResponse::InternalServerError().body("Failed to create user");
        }
    };

    HttpResponse::Created()
        .content_type("application/json")
        .body(serde_json::to_string(&created_user).unwrap_or_else(|_| "{}".to_string()))
}

#[get("/{id}")]
async fn get_user_by_id(
    service: web::Data<service::core::Service>,
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

#[put("/{id}")]
async fn update_user(
    service: web::Data<service::core::Service>,
    user_id: web::Path<String>,
    user: web::Json<UserUpdateReq>,
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
