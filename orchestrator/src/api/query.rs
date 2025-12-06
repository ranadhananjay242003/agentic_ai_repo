use warp::{Rejection, Reply};
use crate::db::DbPool;
use crate::redis_client::RedisClient;
use crate::models::{QueryRequest, QueryResponse};
use uuid::Uuid;
use tracing::{info, error};
use serde_json::json;

pub async fn handle_query(
    request: QueryRequest,
    db_pool: DbPool,
    mut _redis_client: RedisClient,
) -> Result<impl Reply, Rejection> {
    let request_id = Uuid::new_v4();
    info!("Processing query [{}]: {}", request_id, request.query);
    
    let q_lower = request.query.to_lowercase();
    
    // Variables for the response
    let mut pending_action_ids = vec![];
    let mut summary = "I have processed your query against the knowledge base.".to_string();

    // 1. CRITICAL: Insert into 'requests' table first
    // We must do this to satisfy the Foreign Key constraint in 'pending_actions'
    let request_insert = sqlx::query(
        "INSERT INTO requests (id, user_id, query, status, created_at) 
         VALUES ($1, $2, $3, $4, NOW())"
    )
    .bind(request_id)
    .bind(&request.user_id)
    .bind(&request.query)
    .bind("processed") // Initial status
    .execute(&db_pool)
    .await;

    if let Err(e) = request_insert {
        error!("Failed to log request: {}", e);
        // If we can't save the request, we likely can't save the action due to FK.
        // We will return a generic error in the summary.
        return Ok(warp::reply::json(&QueryResponse {
            request_id,
            summary: format!("⚠️ Database Error (Requests Table): {}", e),
            citations: vec![],
            pending_actions: vec![],
        }));
    }
    
    // 2. DECISION RULE: Jira Ticket
    if q_lower.contains("ticket") || q_lower.contains("incident") || q_lower.contains("bug") {
        let action_id = Uuid::new_v4();
        let description = format!("Create JIRA Ticket based on query: '{}'", request.query);
        
        // Pack data into the 'payload' JSONB column
        let payload = json!({
            "description": description,
            "confidence": 0.95,
            "priority": "high"
        });

        // Insert using the CORRECT columns from your screenshot
        let insert_result = sqlx::query(
            "INSERT INTO pending_actions 
            (id, request_id, action_type, target_service, payload, status, created_at) 
             VALUES ($1, $2, $3, $4, $5, $6, NOW())"
        )
        .bind(action_id)
        .bind(request_id)      // Foreign Key
        .bind("JIRA_TICKET")   // action_type
        .bind("jira")          // target_service
        .bind(payload)         // payload (JSONB)
        .bind("pending")       // status
        .execute(&db_pool)
        .await;

        match insert_result {
            Ok(_) => {
                pending_action_ids.push(action_id);
                summary = format!("✅ Success! I have prepared a JIRA ticket for you.\n\nGo to the 'Pending Actions' tab to approve it.\n(Action ID: {})", action_id);
            },
            Err(e) => {
                error!("Failed to save action: {}", e);
                summary = format!("⚠️ Database Error (Action Table): {}", e);
            }
        }
    }
    // 3. DECISION RULE: Email Alert
    else if q_lower.contains("email") || q_lower.contains("alert") {
        let action_id = Uuid::new_v4();
        let description = format!("Send Email Alert: '{}'", request.query);
        
        let payload = json!({
            "description": description,
            "confidence": 0.88,
            "recipient": "security@example.com"
        });

        let insert_result = sqlx::query(
            "INSERT INTO pending_actions 
            (id, request_id, action_type, target_service, payload, status, created_at) 
             VALUES ($1, $2, $3, $4, $5, $6, NOW())"
        )
        .bind(action_id)
        .bind(request_id)
        .bind("EMAIL_ALERT")
        .bind("email")
        .bind(payload)
        .bind("pending")
        .execute(&db_pool)
        .await;

        if insert_result.is_ok() {
            pending_action_ids.push(action_id);
            summary = "✅ I have drafted an email alert for you. Please check the 'Pending Actions' tab.".to_string();
        } else {
             summary = "⚠️ Database error while creating email action.".to_string();
        }
    }

    // Response
    let response = QueryResponse {
        request_id,
        summary,
        citations: vec![], 
        pending_actions: pending_action_ids,
    };
    
    Ok(warp::reply::json(&response))
}