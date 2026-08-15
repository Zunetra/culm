// === The service

pub mod config;
pub mod error;

use actix_web::web::ServiceConfig;
use actix_web::{HttpResponse, get, web};

use crate::error::AppError;

/// Registers every route on an application.
/// Separated from `main` so an integration test drives the real routes in process, rather than a copy of them that can drift.
pub fn routes(config: &mut ServiceConfig) {
    config.service(health).default_service(web::to(not_found));
}

/// Answers the liveness probe.
#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}

/* Registered rather than left to the framework's default, so an unknown path
 * returns the same error body shape as every other failure. */
async fn not_found() -> Result<HttpResponse, AppError> {
    Err(AppError::NotFound)
}
