use warp::Filter;

pub fn cors() -> warp::cors::Builder {
    warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"])
}

// Placeholder for rate limiting - will be implemented with governor
pub fn rate_limit() -> impl Filter<Extract = ((),), Error = warp::Rejection> + Clone {
    warp::any().and_then(|| async { Ok::<(), warp::Rejection>(()) })
}
