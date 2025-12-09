use warp::{Rejection, Reply};
use crate::db::DbPool;
use crate::redis_client::RedisClient;
use uuid::Uuid;
use tracing::{info, error};
use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use serde_json::{json, Value};
// Email Imports
use lettre::{Message, AsyncTransport, Tokio1Executor, AsyncSmtpTransport};
use lettre::message::Mailbox; 
use lettre::transport::smtp::authentication::Credentials;
use std::env;

#[derive(Serialize, FromRow)]
struct PendingActionDTO {
    id: Uuid,
    action_type: String,
    payload: Value,
    status: String,
}

pub async fn handle_get_pending(
    _params: std::collections::HashMap<String, String>,
    db_pool: DbPool,
) -> Result<impl Reply, Rejection> {
    info!("Fetching pending actions...");
    let result = sqlx::query_as::<_, PendingActionDTO>(
        "SELECT id, action_type, payload, status FROM pending_actions WHERE status = 'pending' OR status = 'PENDING' ORDER BY created_at DESC"
    ).fetch_all(&db_pool).await;

    match result {
        Ok(actions) => Ok(warp::reply::json(&actions)),
        Err(e) => {
            error!("Failed to fetch pending actions: {}", e);
            Ok(warp::reply::json(&Vec::<PendingActionDTO>::new()))
        }
    }
}

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

    // 1. Update Database Status
    let update_result = sqlx::query(
        "UPDATE pending_actions SET status = $1, approved_at = NOW(), approved_by = $2 WHERE id = $3"
    ).bind(status).bind(&request.user_signature).bind(request.action_id).execute(&db_pool).await;

    if let Err(e) = update_result {
        return Ok(warp::reply::json(&serde_json::json!({"status": "error", "message": e.to_string()})));
    }

    // 2. EXECUTE REAL ACTION
    if request.approved {
        let action_row = sqlx::query_as::<_, PendingActionDTO>(
            "SELECT id, action_type, payload, status FROM pending_actions WHERE id = $1"
        ).bind(request.action_id).fetch_optional(&db_pool).await;

        if let Ok(Some(action)) = action_row {
            if action.action_type == "EMAIL_ALERT" {
                info!("Executing Email Action...");
                if let Err(e) = send_real_email(&action.payload).await {
                    error!("Failed to send email: {}", e);
                }
            } else if action.action_type == "JIRA_TICKET" {
                info!("Executing Jira Action...");
                if let Err(e) = create_real_jira_ticket(&action.payload).await {
                    error!("Failed to create Jira ticket: {}", e);
                }
            }
        }
    }
    Ok(warp::reply::json(&serde_json::json!({"status": "success"})))
}

// --- HELPER: SEND EMAIL ---
async fn send_real_email(payload: &Value) -> Result<(), String> {
    let smtp_host = env::var("SMTP_HOST").unwrap_or("smtp.gmail.com".to_string());
    let smtp_user = env::var("SMTP_USER").unwrap_or("".to_string());
    let smtp_pass = env::var("SMTP_PASS").unwrap_or("".to_string());

    if smtp_user.is_empty() || smtp_pass.is_empty() { return Err("SMTP creds missing".to_string()); }

    let description = payload["description"].as_str().unwrap_or("No description");
    let recipient = payload["recipient"].as_str().unwrap_or("admin@example.com");

    let email = Message::builder()
        .from(format!("Agentic AI <{}>", smtp_user).parse::<Mailbox>().unwrap())
        .to(recipient.parse::<Mailbox>().map_err(|e| e.to_string())?)
        .subject("🚨 Agentic AI Alert")
        .body(format!("Action executed:\n\n{}", description))
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(smtp_user, smtp_pass);
    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
        .map_err(|e| e.to_string())?.credentials(creds).build();
    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}

// --- HELPER: CREATE JIRA TICKET (NEW) ---
async fn create_real_jira_ticket(payload: &Value) -> Result<(), String> {
    let domain = env::var("JIRA_DOMAIN").unwrap_or_default();
    let user = env::var("JIRA_USER").unwrap_or_default();
    let token = env::var("JIRA_TOKEN").unwrap_or_default();
    let project_key = env::var("JIRA_PROJECT_KEY").unwrap_or("KAN".to_string());

    if domain.is_empty() || user.is_empty() || token.is_empty() {
        return Err("Jira credentials missing in docker-compose".to_string());
    }

    let summary = payload["description"].as_str().unwrap_or("AI Generated Ticket");
    
    // Construct the Jira API URL
    let url = format!("{}/rest/api/3/issue", domain);
    let client = reqwest::Client::new();

    // Jira Cloud requires "Atlassian Document Format" (ADF) for descriptions
    let body = json!({
        "fields": {
            "project": {
                "key": project_key
            },
            "summary": summary,
            "description": {
                "type": "doc",
                "version": 1,
                "content": [
                    {
                        "type": "paragraph",
                        "content": [
                            {
                                "type": "text",
                                "text": format!("This ticket was automatically created by Agentic AI.\n\nDetails: {}", summary)
                            }
                        ]
                    }
                ]
            },
            "issuetype": {
                "name": "Task" 
            }
        }
    });

    info!("Sending request to Jira: {}", url);

    let resp = client.post(url)
        .basic_auth(user, Some(token))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        let resp_json: Value = resp.json().await.map_err(|e| e.to_string())?;
        info!("✅ Jira Ticket Created! Key: {}", resp_json["key"]);
        Ok(())
    } else {
        let error_text = resp.text().await.unwrap_or_default();
        Err(format!("Jira API Error: {}", error_text))
    }
}