// === Errors

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
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

/* One body shape for every failure, so a client parses errors the same way
 * whatever produced them, and an internal message never reaches it. */
impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Config(_) | Self::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let message = match status {
            StatusCode::INTERNAL_SERVER_ERROR => "internal server error".to_owned(),
            _ => self.to_string(),
        };
        HttpResponse::build(status).json(serde_json::json!({ "error": message }))
    }
}
