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
    state: web::Data<State>,
    user_id: web::Path<String>,
    user: web::Json<UserUpdateReq>,
) -> impl Responder {
    let user_uuid_str = &user_id.into_inner();
    let user_uuid = match uuid::Uuid::parse_str(user_uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid UUID format"),
    };

    let new_user_req: UserUpdateReq = user.into_inner();

    let mut users_guard = state.users.lock().unwrap();

    let existing_user = match users_guard.get(&user_uuid) {
        Some(user) => user.clone(),
        None => return HttpResponse::NotFound().body("User not found"),
    };

    let first_name = new_user_req.first_name.unwrap_or(existing_user.first_name);
    let last_name = new_user_req.last_name.unwrap_or(existing_user.last_name);
    let email = new_user_req.email.unwrap_or(existing_user.email);
    let dob = match new_user_req.dob {
        Some(dob_str) => match parse_date(&dob_str) {
            Ok(date) => date,
            Err(e) => {
                println!("{e}");
                return HttpResponse::BadRequest().body("could not parse dob");
            }
        },
        None => existing_user.dob, // Use existing user's dob if not provided
    };

    // Update the user in the hashmap
    users_guard.insert(
        user_uuid,
        service::idos::User::new(first_name, last_name, email, dob),
    );

    HttpResponse::Ok().content_type("application/json").body(
        serde_json::to_string(&users_guard.get(&user_uuid).unwrap())
            .unwrap_or_else(|_| "{}".to_string()),
    )
}

fn parse_date(date_str: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(date_str, "%d-%m-%Y")
        .map_err(|e| format!("Failed to parse date: {}", e))
}
