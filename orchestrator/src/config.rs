use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub openai_api_key: Option<String>,
    pub jwt_secret: String,
    pub ingestion_service_url: String,
    pub embedding_service_url: String,
    pub vector_db_service_url: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        
        Ok(Config {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/agentic_ai".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
            ingestion_service_url: std::env::var("INGESTION_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8001".to_string()),
            embedding_service_url: std::env::var("EMBEDDING_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8002".to_string()),
            vector_db_service_url: std::env::var("VECTOR_DB_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8003".to_string()),
            log_level: std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
        })
    }
}
