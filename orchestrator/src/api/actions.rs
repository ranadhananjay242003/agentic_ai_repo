use warp::{Rejection, Reply};
use crate::db::DbPool;
use crate::redis_client::RedisClient;
use uuid::Uuid;
use tracing::{info, error};
use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use serde_json::Value;

// Define a struct matching the REAL database schema
#[derive(Serialize, FromRow)]
struct PendingActionDTO {
    id: Uuid,
    action_type: String,
    // We map the DB 'payload' (jsonb) column to a serde_json::Value
    payload: Value, 
    status: String,
    // created_at is strictly needed for sorting, handled by sqlx
}

pub async fn handle_get_pending(
    params: std::collections::HashMap<String, String>,
    db_pool: DbPool,
) -> Result<impl Reply, Rejection> {
    // We ignore user_id filter for now to ensure you see ALL actions for debugging
    info!("Fetching pending actions...");

    // 1. SELECT using the CORRECT column names
    let result = sqlx::query_as::<_, PendingActionDTO>(
        "SELECT id, action_type, payload, status 
         FROM pending_actions 
         WHERE status = 'pending' OR status = 'PENDING'
         ORDER BY created_at DESC"
    )
    .fetch_all(&db_pool)
    .await;

    match result {
        Ok(actions) => {
            info!("Found {} pending actions", actions.len());
            Ok(warp::reply::json(&actions))
        },
        Err(e) => {
            error!("Failed to fetch pending actions: {}", e);
            // Return empty list on error so frontend doesn't crash
            Ok(warp::reply::json(&Vec::<PendingActionDTO>::new()))
        }
    }
}

// Request struct for approval
#[derive(Deserialize)]
pub struct ApproveRequest {
    pub action_id: Uuid,
    pub approved: bool,
    pub user_signature: String,
}

pub async fn handle_approve(
    request: ApproveRequest,
    db_pool: DbPool,
    mut _redis_client: RedisClient,
) -> Result<impl Reply, Rejection> {
    info!("Processing approval for action {}", request.action_id);

    let status = if request.approved { "approved" } else { "rejected" };

    // Update status in DB
    let result = sqlx::query(
        "UPDATE pending_actions 
         SET status = $1, approved_at = NOW(), approved_by = $2 
         WHERE id = $3"
    )
    .bind(status)
    .bind(&request.user_signature)
    .bind(request.action_id)
    .execute(&db_pool)
    .await;

    match result {
        Ok(_) => Ok(warp::reply::json(&serde_json::json!({"status": "success"}))),
        Err(e) => {
            error!("Failed to update action: {}", e);
            Ok(warp::reply::json(&serde_json::json!({"status": "error", "message": e.to_string()})))
        }
    }
}