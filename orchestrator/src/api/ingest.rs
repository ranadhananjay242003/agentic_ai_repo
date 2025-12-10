use warp::{Rejection, Reply, multipart::{FormData, Part}};
use crate::db::DbPool;
use crate::models::{IngestResponse};
use uuid::Uuid;
use futures::{StreamExt, TryStreamExt};
use tracing::{info, error};
use serde_json::{json, Value};
use bytes::Buf;
use std::env;

pub async fn handle_ingest(
    mut form: FormData,
    db_pool: DbPool,
) -> Result<impl Reply, Rejection> {
    info!("Starting document ingestion...");

    let mut filename = String::from("unknown_file");
    let mut content_type = String::from("application/octet-stream");
    let mut file_bytes = Vec::new();
    let mut user_id = String::from("admin_user"); // Default fallback

    while let Ok(Some(part)) = form.try_next().await {
        let name = part.name().to_string();
        
        if name == "file" {
            filename = part.filename().unwrap_or("unknown").to_string();
            content_type = part.content_type().unwrap_or("application/octet-stream").to_string();
            let data = part.stream().try_fold(Vec::new(), |mut vec, data| async move {
                vec.extend_from_slice(data.chunk());
                Ok(vec)
            }).await.map_err(|_| warp::reject::not_found())?;
            file_bytes = data;
        } else if name == "user_id" {
            // Extract user_id string from form field
            let data = part.stream().try_fold(Vec::new(), |mut vec, data| async move {
                vec.extend_from_slice(data.chunk());
                Ok(vec)
            }).await.map_err(|_| warp::reject::not_found())?;
            user_id = String::from_utf8(data).unwrap_or("admin_user".to_string());
        }
    }

    if file_bytes.is_empty() {
        return Ok(warp::reply::json(&json!({"error": "No file uploaded"})));
    }

    info!("Ingesting file '{}' for User: {}", filename, user_id);
    let client = reqwest::Client::new();

    // 1. EXTRACT
    let ingest_url = env::var("INGESTION_SERVICE_URL").unwrap_or("http://ingestion-service:8001".to_string());
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename.clone())
        .mime_str(&content_type)
        .map_err(|_| warp::reject::not_found())?;
    
    let multipart_form = reqwest::multipart::Form::new().part("file", part);
    let ingest_res = client.post(format!("{}/extract", ingest_url))
        .multipart(multipart_form)
        .send().await.map_err(|_| warp::reject::not_found())?;

    let extraction_data: Value = ingest_res.json().await.map_err(|_| warp::reject::not_found())?;
    let passages = extraction_data["passages"].as_array().ok_or_else(warp::reject::not_found)?;
    let total_chars = extraction_data["total_chars"].as_u64().unwrap_or(0);

    // 2. SAVE DB
    let doc_id = Uuid::new_v4();
    let _ = sqlx::query("INSERT INTO documents (id, filename, content_type, s3_key, upload_time, user_id, metadata) VALUES ($1, $2, $3, $4, NOW(), $5, $6)")
        .bind(doc_id).bind(&filename).bind(&content_type).bind("local").bind(&user_id).bind(json!({ "total_chars": total_chars as i64 }))
        .execute(&db_pool).await;

    // 3. EMBED & INDEX
    let mut texts_to_embed = Vec::new();
    let mut metadatas = Vec::new();

    for (i, p) in passages.iter().enumerate() {
        let text = p["text"].as_str().unwrap_or("").to_string();
        let page = p["page"].as_i64();
        
        if !text.trim().is_empty() {
            texts_to_embed.push(text.clone());
            metadatas.push(json!({
                "text": text,
                "doc_id": doc_id.to_string(),
                "page": page,
                "filename": filename,
                "user_id": user_id  // <--- CRITICAL: ATTACH USER ID TO VECTOR
            }));
        }
    }

    if !texts_to_embed.is_empty() {
        let embed_url = env::var("EMBEDDING_SERVICE_URL").unwrap_or("http://embedding-service:8002".to_string());
        let vector_url = env::var("VECTOR_DB_SERVICE_URL").unwrap_or("http://vector-db-service:8003".to_string());

        for (chunk_texts, chunk_metas) in texts_to_embed.chunks(50).zip(metadatas.chunks(50)) {
            let embed_req = json!({ "texts": chunk_texts });
            if let Ok(resp) = client.post(format!("{}/embed", embed_url)).json(&embed_req).send().await {
                if let Ok(embed_data) = resp.json::<Value>().await {
                    if let Some(embeddings) = embed_data["embeddings"].as_array() {
                        let add_req = json!({ "vectors": embeddings, "metadata": chunk_metas });
                        let _ = client.post(format!("{}/index/add", vector_url)).json(&add_req).send().await;
                    }
                }
            }
        }
    }

    let response = IngestResponse { document_id: doc_id, filename, passages_count: passages.len() };
    Ok(warp::reply::json(&response))
}