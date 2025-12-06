use warp::Filter;
use tracing::{info, instrument};

mod agents;
mod api;
mod config;
mod db;
mod error;
mod middleware;
mod models;
mod redis_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .json()
        .init();

    info!("Starting Agentic AI Knowledge Workflow Orchestrator");

    // Load configuration
    let config = config::Config::from_env()?;
    info!("Configuration loaded");

    // Initialize database pool
    let db_pool = db::create_pool(&config.database_url).await?;
    info!("Database connection pool created");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await?;
    info!("Database migrations applied");

    // Initialize Redis client
    let redis_client = redis_client::RedisClient::new(&config.redis_url).await?;
    info!("Redis connection established");

    // Build API routes
    let api_routes = api::routes(db_pool.clone(), redis_client.clone())
        .with(warp::log("api"))
        .with(middleware::cors());

    // Health check route
    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::json(&serde_json::json!({"status": "healthy"})));

    // Metrics route
    let metrics = warp::path("metrics")
        .and(warp::get())
        .map(|| {
            use prometheus::{Encoder, TextEncoder};
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = vec![];
            encoder.encode(&metric_families, &mut buffer).unwrap();
            warp::reply::with_header(
                buffer,
                "Content-Type",
                encoder.format_type(),
            )
        });

    let routes = health
        .or(metrics)
        .or(api_routes);

    // Start server
    let addr = ([0, 0, 0, 0], config.port);
    info!("Server listening on {}", addr.1);

    warp::serve(routes)
        .run(addr)
        .await;

    Ok(())
}
