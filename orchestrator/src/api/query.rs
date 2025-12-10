use warp::{Rejection, Reply};
use crate::db::DbPool;
use crate::redis_client::RedisClient;
use crate::models::{QueryRequest, QueryResponse, Citation};
use uuid::Uuid;
use tracing::{info, error, warn};
use serde_json::{json, Value};
use std::env;

pub async fn handle_query(
    request: QueryRequest,
    db_pool: DbPool,
    mut _redis_client: RedisClient,
) -> Result<impl Reply, Rejection> {
    let request_id = Uuid::new_v4();
    info!("Processing query for User {}: {}", request.user_id, request.query);
    
    // 1. Log Request
    let _ = sqlx::query("INSERT INTO requests (id, user_id, query, status, created_at) VALUES ($1, $2, $3, $4, NOW())")
        .bind(request_id).bind(&request.user_id).bind(&request.query).bind("processing").execute(&db_pool).await;

    let q_lower = request.query.to_lowercase();
    let mut pending_action_ids = vec![];
    let client = reqwest::Client::new();
    
    // --- PATH A: DECISION LOGIC (Tickets) ---
    if q_lower.contains("ticket") || q_lower.contains("incident") {
        let action_id = Uuid::new_v4();
        let payload = json!({ "description": format!("Create JIRA Ticket: '{}'", request.query), "priority": "high" });
        let _ = sqlx::query("INSERT INTO pending_actions (id, request_id, action_type, target_service, payload, status, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW())")
            .bind(action_id).bind(request_id).bind("JIRA_TICKET").bind("jira").bind(payload).bind("pending").execute(&db_pool).await;
        pending_action_ids.push(action_id);
        return Ok(warp::reply::json(&QueryResponse { request_id, summary: format!("✅ Prepared JIRA ticket (Action ID: {})", action_id), citations: vec![], pending_actions: pending_action_ids }));
    } 
    
    // --- PATH B: DECISION LOGIC (Emails) ---
    else if q_lower.contains("email") || q_lower.contains("alert") {
        let action_id = Uuid::new_v4();
        let payload = json!({ "description": format!("Send Email: '{}'", request.query), "recipient": "admin@example.com", "priority": "high" });
        let _ = sqlx::query("INSERT INTO pending_actions (id, request_id, action_type, target_service, payload, status, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW())")
            .bind(action_id).bind(request_id).bind("EMAIL_ALERT").bind("smtp").bind(payload).bind("pending").execute(&db_pool).await;
        pending_action_ids.push(action_id);
        return Ok(warp::reply::json(&QueryResponse { request_id, summary: format!("✅ Drafted Email Alert (Action ID: {})", action_id), citations: vec![], pending_actions: pending_action_ids }));
    }
    
    // --- PATH B: DECISION LOGIC (Slack) --- <--- NEW SLACK BLOCK
    else if q_lower.contains("slack") || q_lower.contains("post to channel") {
        let action_id = Uuid::new_v4();
        let payload = json!({ 
            "description": format!("Post to Slack Channel: '{}'", request.query), 
            "channel": "#general",
            "priority": "high" 
        });
        
        let _ = sqlx::query("INSERT INTO pending_actions (id, request_id, action_type, target_service, payload, status, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW())")
            .bind(action_id).bind(request_id).bind("SLACK_ALERT").bind("slack").bind(payload).bind("pending")
            .execute(&db_pool).await;
            
        pending_action_ids.push(action_id);
        
        return Ok(warp::reply::json(&QueryResponse {
            request_id,
            summary: format!("✅ I have drafted a Slack message. Check the 'Pending Actions' tab to approve and post it."),
            citations: vec![],
            pending_actions: pending_action_ids,
        }));
    }

    // --- PATH C: SANDBOXED CODE EXECUTION (Math/Logic) ---
    else if q_lower.contains("calculate") || q_lower.contains("solve") || q_lower.contains("math") {
        let groq_api_key = env::var("GROQ_API_KEY").unwrap_or_default();
        if !groq_api_key.is_empty() {
            let code_prompt = json!({
                "model": "llama-3.3-70b-versatile",
                "messages": [
                    { "role": "system", "content": "You are a Python Coding Assistant. Output ONLY valid Python code to solve the user's problem. Use 'print()' to output the final answer." },
                    { "role": "user", "content": request.query }
                ]
            });
            if let Ok(resp) = client.post("https://api.groq.com/openai/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", groq_api_key)).json(&code_prompt).send().await {
                    if let Ok(json_resp) = resp.json::<Value>().await {
                        if let Some(python_code) = json_resp["choices"][0]["message"]["content"].as_str() {
                            let clean_code = python_code.replace("```python", "").replace("```", "").trim().to_string();
                            let interpreter_url = "http://code-interpreter:8004/execute";
                            if let Ok(exec_res) = client.post(interpreter_url).json(&json!({ "code": clean_code })).send().await {
                                    if let Ok(exec_data) = exec_res.json::<Value>().await {
                                        let output = exec_data["output"].as_str().unwrap_or("No output").to_string();
                                        // Final output string for frontend
                                        let summary = format!("🤖 **I wrote and executed a Python script to calculate this:**\n\nCode:\n```python\n{}\n```\n\nResult:\n```\n{}\n```", clean_code, output);
                                        return Ok(warp::reply::json(&QueryResponse { request_id, summary, citations: vec![], pending_actions: vec![] }));
                                    }
                            }
                        }
                    }
            }
        }
    }

    // --- PATH D: STANDARD RAG (Fallback) ---
    let embedding_url = env::var("EMBEDDING_SERVICE_URL").unwrap_or("http://embedding-service:8002".to_string());
    let vector_url = env::var("VECTOR_DB_SERVICE_URL").unwrap_or("http://vector-db-service:8003".to_string());
    let mut context_text = String::new();
    let mut citations = Vec::new();

    if let Ok(resp) = client.post(format!("{}/embed", embedding_url)).json(&json!({ "texts": [request.query] })).send().await {
        if let Ok(json_data) = resp.json::<Value>().await {
            if let Some(vecs) = json_data["embeddings"].as_array() {
                if let Some(first_vec) = vecs.get(0).and_then(|v| v.as_array()) {
                    let vector: Vec<f64> = first_vec.iter().map(|n| n.as_f64().unwrap_or(0.0)).collect();
                    let search_payload = json!({ "query_vector": vector, "query_text": request.query, "top_k": 3, "hybrid": true, "user_id": request.user_id });
                    if let Ok(s_resp) = client.post(format!("{}/search/hybrid", vector_url)).json(&search_payload).send().await {
                        if let Ok(results) = s_resp.json::<Value>().await {
                            if let Some(matches) = results["results"].as_array() {
                                for m in matches {
                                    let text = m["metadata"]["text"].as_str().unwrap_or("").to_string();
                                    let score = m["score"].as_f64().unwrap_or(0.0) as f32;
                                    if !text.is_empty() {
                                        context_text.push_str(&format!("- {}\n", text));
                                        citations.push(Citation { doc_id: Uuid::new_v4(), passage_id: Uuid::new_v4(), page: m["metadata"]["page"].as_i64().map(|v| v as i32), text: text.chars().take(150).collect(), relevance_score: score });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if context_text.is_empty() { context_text = "No relevant documents found.".to_string(); }

    let groq_api_key = env::var("GROQ_API_KEY").unwrap_or_default();
    let mut summary = String::new();

    if !groq_api_key.is_empty() {
        let llm_body = json!({
            "model": "llama-3.3-70b-versatile",
            "messages": [
                { "role": "system", "content": "You are a helpful Enterprise AI. Use the provided Context to answer." },
                { "role": "user", "content": format!("Context:\n{}\n\nQuestion: {}", context_text, request.query) }
            ]
        });
        if let Ok(resp) = client.post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", groq_api_key)).header("Content-Type", "application/json").json(&llm_body).send().await {
                if let Ok(json_resp) = resp.json::<Value>().await {
                    summary = json_resp["choices"][0]["message"]["content"].as_str().unwrap_or("Error").to_string();
                }
        }
    }

    Ok(warp::reply::json(&QueryResponse { request_id, summary, citations, pending_actions: vec![] }))
}