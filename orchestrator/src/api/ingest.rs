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

    // 1. EXTRACT FILE FROM FORM DATA
    let mut filename = String::from("unknown_file");
    let mut content_type = String::from("application/octet-stream");
    let mut file_bytes = Vec::new();

    while let Ok(Some(part)) = form.try_next().await {
        if part.name() == "file" {
            filename = part.filename().unwrap_or("unknown").to_string();
            content_type = part.content_type().unwrap_or("application/octet-stream").to_string();
            
            // Read bytes from stream
            let data = part.stream().try_fold(Vec::new(), |mut vec, data| async move {
                vec.extend_from_slice(data.chunk());
                Ok(vec)
            }).await.map_err(|e| {
                error!("Error reading file bytes: {}", e);
                warp::reject::not_found() // Generic error mapping
            })?;
            
            file_bytes = data;
        }
    }

    if file_bytes.is_empty() {
        return Ok(warp::reply::json(&json!({"error": "No file uploaded"})));
    }

    info!("Received file: {} ({} bytes)", filename, file_bytes.len());
    let client = reqwest::Client::new();

    // 2. CALL INGESTION SERVICE (Extract Text)
    let ingest_url = env::var("INGESTION_SERVICE_URL").unwrap_or("http://ingestion-service:8001".to_string());
    
    // Create multipart form for the python service
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename.clone())
        .mime_str(&content_type)
        .map_err(|_| warp::reject::not_found())?;

    let multipart_form = reqwest::multipart::Form::new().part("file", part);

    info!("Sending to Ingestion Service at {}...", ingest_url);
    let ingest_res = client.post(format!("{}/extract", ingest_url))
        .multipart(multipart_form)
        .send()
        .await
        .map_err(|e| { error!("Ingestion Service failed: {}", e); warp::reject::not_found() })?;

    if !ingest_res.status().is_success() {
        error!("Ingestion Service returned error: {}", ingest_res.status());
        return Ok(warp::reply::json(&json!({"error": "Ingestion service failed"})));
    }

    let extraction_data: Value = ingest_res.json().await.map_err(|_| warp::reject::not_found())?;
    let passages = extraction_data["passages"].as_array().ok_or_else(warp::reject::not_found)?;
    let total_chars = extraction_data["total_chars"].as_u64().unwrap_or(0);

    info!("Extracted {} passages ({} chars)", passages.len(), total_chars);

    // 3. SAVE TO POSTGRES (Permanent Record)
    let doc_id = Uuid::new_v4();
    
    // Save Document Metadata
    sqlx::query(
        "INSERT INTO documents (id, filename, content_type, s3_key, upload_time, user_id, metadata) 
         VALUES ($1, $2, $3, $4, NOW(), $5, $6)"
    )
    .bind(doc_id)
    .bind(&filename)
    .bind(&content_type)
    .bind("local_storage") // S3 placeholder
    .bind("admin_user")    // MVP user
    .bind(json!({ "total_chars": total_chars as i64 }))
    .execute(&db_pool)
    .await
    .map_err(|e| { error!("DB Error: {}", e); warp::reject::not_found() })?;

    // 4. PREPARE FOR EMBEDDING
    let mut texts_to_embed = Vec::new();
    let mut metadatas = Vec::new();

    for (i, p) in passages.iter().enumerate() {
        let text = p["text"].as_str().unwrap_or("").to_string();
        let page = p["page"].as_i64();
        
        // Save Passage to Postgres
        // Note: We skip error checking on individual inserts to speed up
        let _ = sqlx::query(
            "INSERT INTO passages (id, doc_id, passage_index, text, char_start, char_end, page_num, metadata) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(Uuid::new_v4())
        .bind(doc_id)
        .bind(i as i32)
        .bind(&text)
        .bind(0) // Simplified
        .bind(0) // Simplified
        .bind(page.map(|v| v as i32))
        .bind(json!({}))
        .execute(&db_pool)
        .await;

        // Collect for Vector DB
        if !text.trim().is_empty() {
            texts_to_embed.push(text.clone());
            metadatas.push(json!({
                "text": text, // Important: Store text in metadata so we can retrieve it later!
                "doc_id": doc_id.to_string(),
                "page": page,
                "filename": filename
            }));
        }
    }

    // 5. CALL EMBEDDING SERVICE
    if !texts_to_embed.is_empty() {
        let embed_url = env::var("EMBEDDING_SERVICE_URL").unwrap_or("http://embedding-service:8002".to_string());
        info!("Embedding {} chunks...", texts_to_embed.len());

        // Process in batches of 50 to allow larger files
        for (chunk_texts, chunk_metas) in texts_to_embed.chunks(50).zip(metadatas.chunks(50)) {
            let embed_req = json!({ "texts": chunk_texts });
            
            let embed_res = client.post(format!("{}/embed", embed_url))
                .json(&embed_req)
                .send()
                .await;

            if let Ok(resp) = embed_res {
                if let Ok(embed_data) = resp.json::<Value>().await {
                    if let Some(embeddings) = embed_data["embeddings"].as_array() {
                        
                        // 6. SAVE TO VECTOR DB
                        let vector_url = env::var("VECTOR_DB_SERVICE_URL").unwrap_or("http://vector-db-service:8003".to_string());
                        
                        let add_req = json!({
                            "vectors": embeddings,
                            "metadata": chunk_metas
                        });

                        let vec_res = client.post(format!("{}/index/add", vector_url))
                            .json(&add_req)
                            .send()
                            .await;
                            
                        match vec_res {
                            Ok(_) => info!("Indexed batch of {} vectors", embeddings.len()),
                            Err(e) => error!("Failed to save to Vector DB: {}", e),
                        }
                    }
                }
            } else {
                error!("Embedding service failed for batch");
            }
        }
    }

    let response = IngestResponse {
        document_id: doc_id,
        filename,
        passages_count: passages.len(),
    };
    
    Ok(warp::reply::json(&response))
}