use lambda_http::{Body, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("resource not found")]
    NotFound,
    #[error("resource conflict: {0}")]
    Conflict(String),
    #[error("method not allowed")]
    MethodNotAllowed,
    #[error("authentication required")]
    Unauthorized,
    #[error("permission denied")]
    Forbidden,
    #[error("dependent service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("database operation failed")]
    Database(#[from] mongodb::error::Error),
    #[error("serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid persisted data: {0}")]
    InvalidData(String),
}

impl AppError {
    pub fn into_response(self) -> Response<Body> {
        let (status, code, message) = match &self {
            Self::BadRequest(message) => (400, "bad_request", message.as_str()),
            Self::NotFound => (404, "not_found", "Product not found"),
            Self::Conflict(message) => (409, "conflict", message.as_str()),
            Self::MethodNotAllowed => (405, "method_not_allowed", "Method not allowed"),
            Self::Unauthorized => (401, "unauthorized", "Authentication required"),
            Self::Forbidden => (403, "forbidden", "Permission denied"),
            Self::ServiceUnavailable(_) => {
                tracing::error!(error = %self, "authentication dependency unavailable");
                (
                    503,
                    "service_unavailable",
                    "Authentication service temporarily unavailable",
                )
            }
            Self::Configuration(_)
            | Self::Database(_)
            | Self::Serialization(_)
            | Self::InvalidData(_) => {
                tracing::error!(error = %self, "request failed");
                (500, "internal_error", "An internal error occurred")
            }
        };

        Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Body::Text(
                json!({ "error": { "code": code, "message": message } }).to_string(),
            ))
            .expect("valid error response")
    }
}
