use warp::{Filter, Rejection, Reply};
use crate::db::DbPool;
use crate::redis_client::RedisClient;

mod ingest;
mod query;
mod actions;
mod sources;

pub fn routes(
    db_pool: DbPool,
    redis_client: RedisClient,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let api = warp::path("api").and(warp::path("v1"));

    let ingest_route = api
        .and(warp::path("ingest"))
        .and(warp::post())
        .and(warp::multipart::form().max_length(100 * 1024 * 1024)) // 100MB max
        .and(with_db(db_pool.clone()))
        .and_then(ingest::handle_ingest);

    let query_route = api
        .and(warp::path("query"))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db(db_pool.clone()))
        .and(with_redis(redis_client.clone()))
        .and_then(query::handle_query);

    let pending_route = api
        .and(warp::path("pending"))
        .and(warp::get())
        .and(warp::query())
        .and(with_db(db_pool.clone()))
        .and_then(actions::handle_get_pending);

    let approve_route = api
        .and(warp::path("approve"))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db(db_pool.clone()))
        .and(with_redis(redis_client.clone()))
        .and_then(actions::handle_approve);

    let sources_route = api
        .and(warp::path("sources"))
        .and(warp::path::param())
        .and(warp::get())
        .and(with_db(db_pool.clone()))
        .and_then(sources::handle_get_source);

    ingest_route
        .or(query_route)
        .or(pending_route)
        .or(approve_route)
        .or(sources_route)
}

fn with_db(
    db_pool: DbPool,
) -> impl Filter<Extract = (DbPool,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db_pool.clone())
}

fn with_redis(
    redis_client: RedisClient,
) -> impl Filter<Extract = (RedisClient,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || redis_client.clone())
}
