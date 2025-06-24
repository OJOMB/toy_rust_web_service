use std::sync::Arc;

use actix_web::middleware::NormalizePath;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use tracing_subscriber::fmt::format::FmtSpan;

mod users;

// This struct represents shared application-wide state.
struct AppState {
    app_name: String,
    version: String,
}

#[get("/health")]
async fn health_check(state: web::Data<AppState>) -> impl Responder {
    let response_str = format!("App Name: {}, Version: {}", state.app_name, state.version);
    HttpResponse::Ok().body(response_str)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .json()
        .init();

    tracing::info!("booting application");

    // initialise the user repo
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    // TODO: this should be handled by config
    let table_name = "users".to_string();
    let email_lookup_table_name = "users_email_lookup".to_string();

    let repo = users::repo::dynamodb::Repo::new(client, table_name, email_lookup_table_name);
    let users_service = users::service::core::Service::new(repo);
    let users_app = users::app::core::App::new("/api/users".to_string(), Arc::new(users_service));

    HttpServer::new(move || {
        App::new()
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(AppState {
                app_name: "My Actix Web App".to_string(),
                version: "1.0.0".to_string(),
            }))
            .configure(users_app.configure())
            .service(health_check)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
