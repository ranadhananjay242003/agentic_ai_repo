use warp::Filter;

pub fn cors() -> warp::cors::Builder {
    warp::cors()
        .allow_any_origin() // Allows Vercel, Localhost, etc.
        .allow_headers(vec![
            "User-Agent", 
            "Sec-Fetch-Mode", 
            "Referer", 
            "Origin", 
            "Access-Control-Request-Method", 
            "Access-Control-Request-Headers", 
            "content-type", 
            "authorization"
        ])
        .allow_methods(vec!["POST", "GET", "OPTIONS", "DELETE"])
}