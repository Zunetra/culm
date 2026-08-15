// === The service

pub mod config;
pub mod error;

use axum::routing::get;
use axum::{Json, Router};

use crate::error::AppError;

/// Builds the router without binding a port.
/// Separated from `main` so an integration test drives the real routes in process, rather than a copy of them that can drift.
pub fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .fallback(not_found)
}

/// Answers the liveness probe.
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/* Routed rather than left to the framework's default, so an unknown path
 * returns the same error body shape as every other failure. */
async fn not_found() -> AppError {
    AppError::NotFound
}
