use thiserror::Error;
use warp::{reject::Reject, Reply, Rejection};

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Authentication failed")]
    AuthenticationError,
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

impl Reject for ApiError {}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(api_err) = err.find::<ApiError>() {
        let (code, message) = match api_err {
            ApiError::AuthenticationError => (401, "Authentication failed"),
            ApiError::NotFound(_) => (404, "Resource not found"),
            ApiError::BadRequest(_) => (400, "Bad request"),
            ApiError::RateLimitExceeded => (429, "Rate limit exceeded"),
            _ => (500, "Internal server error"),
        };

        let json = warp::reply::json(&serde_json::json!({
            "error": message,
            "details": api_err.to_string(),
        }));

        Ok(warp::reply::with_status(json, warp::http::StatusCode::from_u16(code).unwrap()))
    } else {
        Err(err)
    }
}
