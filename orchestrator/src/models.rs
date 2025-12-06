use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Document {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub s3_key: String,
    pub upload_time: DateTime<Utc>,
    pub user_id: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Passage {
    pub id: Uuid,
    pub doc_id: Uuid,
    pub passage_index: i32,
    pub text: String,
    pub char_start: i32,
    pub char_end: i32,
    pub page_num: Option<i32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EmbeddingMeta {
    pub id: Uuid,
    pub passage_id: Uuid,
    pub embedding_model: String,
    pub vector_db_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Request {
    pub id: Uuid,
    pub user_id: String,
    pub query: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub request_id: Uuid,
    pub agent_type: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingAction {
    pub id: Uuid,
    pub request_id: Uuid,
    pub action_type: String,
    pub target_service: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub request_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub event_type: String,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
}

// API Request/Response models
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub document_id: Uuid,
    pub filename: String,
    pub passages_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub user_id: String,
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub request_id: Uuid,
    pub summary: String,
    pub citations: Vec<Citation>,
    pub pending_actions: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Citation {
    pub doc_id: Uuid,
    pub passage_id: Uuid,
    pub page: Option<i32>,
    pub text: String,
    pub relevance_score: f32,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalRequest {
    pub action_id: Uuid,
    pub approved: bool,
    pub user_signature: String,
}

#[derive(Debug, Serialize)]
pub struct ApprovalResponse {
    pub action_id: Uuid,
    pub executed: bool,
    pub result: Option<serde_json::Value>,
}

// Agent communication models
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannerStep {
    pub step: i32,
    pub action: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub passages: Vec<Passage>,
    pub embeddings: Vec<EmbeddingMeta>,
    pub scores: Vec<f32>,
}
