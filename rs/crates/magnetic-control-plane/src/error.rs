use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Database(String),
    Unauthorized,
    Forbidden(String),
    NotFound(String),
    BadRequest(String),
    Internal(String),
    Upstream(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database: {}", e),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::Forbidden(e) => write!(f, "forbidden: {}", e),
            Self::NotFound(e) => write!(f, "not found: {}", e),
            Self::BadRequest(e) => write!(f, "bad request: {}", e),
            Self::Internal(e) => write!(f, "internal: {}", e),
            Self::Upstream(e) => write!(f, "upstream: {}", e),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Self::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.as_str()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "invalid or missing API key"),
            Self::Forbidden(e) => (StatusCode::FORBIDDEN, e.as_str()),
            Self::NotFound(e) => (StatusCode::NOT_FOUND, e.as_str()),
            Self::BadRequest(e) => (StatusCode::BAD_REQUEST, e.as_str()),
            Self::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.as_str()),
            Self::Upstream(e) => (StatusCode::BAD_GATEWAY, e.as_str()),
        };
        let body = serde_json::json!({ "error": msg });
        (status, axum::Json(body)).into_response()
    }
}

impl From<libsql::Error> for AppError {
    fn from(e: libsql::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::Upstream(e.to_string())
    }
}
