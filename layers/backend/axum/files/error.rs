// === Errors

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use thiserror::Error;

use crate::config::ConfigError;

/// Every way a request or a startup can fail.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("the service is misconfigured: {0}")]
    Config(#[from] ConfigError),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("not found")]
    NotFound,
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Config(_) | Self::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/* One body shape for every failure, so a client parses errors the same way
 * whatever produced them, and an internal message never reaches it. */
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let message = match status {
            StatusCode::INTERNAL_SERVER_ERROR => "internal server error".to_owned(),
            _ => self.to_string(),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
