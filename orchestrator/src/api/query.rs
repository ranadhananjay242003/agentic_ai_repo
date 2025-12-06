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
    info!("Processing query [{}]: {}", request_id, request.query);
    
    // 1. Log Request
    let _ = sqlx::query(
        "INSERT INTO requests (id, user_id, query, status, created_at) VALUES ($1, $2, $3, $4, NOW())"
    )
    .bind(request_id)
    .bind(&request.user_id)
    .bind(&request.query)
    .bind("processing")
    .execute(&db_pool)
    .await;

    let q_lower = request.query.to_lowercase();
    let mut pending_action_ids = vec![];
    
    // --- PATH A: HEURISTIC ACTIONS ---
    if q_lower.contains("ticket") || q_lower.contains("incident") {
        let action_id = Uuid::new_v4();
        let payload = json!({ "description": format!("Create JIRA Ticket: '{}'", request.query), "priority": "high" });
        
        let _ = sqlx::query("INSERT INTO pending_actions (id, request_id, action_type, target_service, payload, status, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW())")
            .bind(action_id).bind(request_id).bind("JIRA_TICKET").bind("jira").bind(payload).bind("pending")
            .execute(&db_pool).await;
            
        pending_action_ids.push(action_id);
        
        return Ok(warp::reply::json(&QueryResponse {
            request_id,
            summary: format!("✅ I have prepared a JIRA ticket. (Action ID: {})", action_id),
            citations: vec![],
            pending_actions: pending_action_ids,
        }));
    }

    // --- PATH B: REAL AI (RAG + Groq) ---
    
    let client = reqwest::Client::new();
    let embedding_service_url = env::var("EMBEDDING_SERVICE_URL").unwrap_or("http://embedding-service:8002".to_string());
    let vector_service_url = env::var("VECTOR_DB_SERVICE_URL").unwrap_or("http://vector-db-service:8003".to_string());

    let mut context_text = String::new();
    let mut citations = Vec::new();

    // STEP 1: Embed the User's Query (Text -> Numbers)
    info!("Embedding query...");
    let embed_res = client.post(format!("{}/embed", embedding_service_url))
        .json(&json!({ "texts": [request.query] }))
        .send()
        .await;

    let mut query_vector: Option<Vec<f64>> = None;

    match embed_res {
        Ok(resp) => {
            if let Ok(json_data) = resp.json::<Value>().await {
                if let Some(vecs) = json_data["embeddings"].as_array() {
                    if let Some(first_vec) = vecs.get(0).and_then(|v| v.as_array()) {
                         // Convert JSON numbers to Rust f64 vector
                         let v: Vec<f64> = first_vec.iter().map(|n| n.as_f64().unwrap_or(0.0)).collect();
                         query_vector = Some(v);
                    }
                }
            }
        }
        Err(e) => error!("Embedding service failed: {}", e),
    }

    // STEP 2: Search Vector DB (using the vector we just got)
    if let Some(vector) = query_vector {
        info!("Searching Vector DB with vector...");
        
        let search_res = client.post(format!("{}/search/hybrid", vector_service_url)) // <--- FIXED URL
            .json(&json!({ 
                "query_vector": vector,
                "query_text": request.query,
                "top_k": 3,
                "hybrid": true
            }))
            .send()
            .await;

        match search_res {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(results) = resp.json::<Value>().await {
                        if let Some(matches) = results["results"].as_array() {
                            for m in matches {
                                let metadata = &m["metadata"];
                                let text = metadata["text"].as_str().unwrap_or("").to_string();
                                let score = m["score"].as_f64().unwrap_or(0.0) as f32;
                                
                                if !text.is_empty() {
                                    context_text.push_str(&format!("- {}\n", text));
                                    citations.push(Citation {
                                        doc_id: Uuid::new_v4(), 
                                        passage_id: Uuid::new_v4(),
                                        page: metadata["page"].as_i64().map(|v| v as i32),
                                        text: text.chars().take(150).collect(),
                                        relevance_score: score,
                                    });
                                }
                            }
                        }
                    }
                } else {
                    warn!("Vector DB returned error: {}", resp.status());
                }
            },
            Err(e) => warn!("Vector DB Unreachable: {}", e),
        }
    } else {
        warn!("Skipping vector search because embedding failed.");
    }

    if context_text.is_empty() {
        context_text = "No specific documents found. Answer based on general knowledge.".to_string();
    } else {
        info!("RAG Context found ({} chars)", context_text.len());
    }

    // STEP 3: Call Groq
    let groq_api_key = env::var("GROQ_API_KEY").unwrap_or_default();
    let mut summary = String::new();

    if !groq_api_key.is_empty() {
        let system_prompt = "You are an Enterprise AI. Answer using the provided Context. If the context is empty, use general knowledge. Be concise.";
        
        let llm_body = json!({
            "model": "llama-3.3-70b-versatile",
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": format!("Context:\n{}\n\nUser Question: {}", context_text, request.query) }
            ]
        });

        let llm_res = client.post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", groq_api_key))
            .header("Content-Type", "application/json")
            .json(&llm_body)
            .send()
            .await;

        match llm_res {
            Ok(resp) => {
                if let Ok(json_resp) = resp.json::<Value>().await {
                    if let Some(content) = json_resp["choices"][0]["message"]["content"].as_str() {
                        summary = content.to_string();
                    } else {
                        summary = format!("⚠️ Groq Error: {:?}", json_resp);
                    }
                }
            },
            Err(e) => {
                summary = format!("⚠️ Network Error: {}", e);
            }
        }
    } else {
        summary = "⚠️ GROQ_API_KEY is missing.".to_string();
    }

    Ok(warp::reply::json(&QueryResponse {
        request_id,
        summary,
        citations, 
        pending_actions: vec![],
    }))
}