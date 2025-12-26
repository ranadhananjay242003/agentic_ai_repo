use warp::Filter;

pub fn cors() -> warp::cors::Builder {
    warp::cors()
        .allow_any_origin()
        // Allow ALL headers (The "*" is sometimes restricted, so we list the vital ones plus the missing ones)
        .allow_headers(vec![
            "User-Agent", 
            "Sec-Fetch-Mode", 
            "Referer", 
            "Origin", 
            "Access-Control-Request-Method", 
            "Access-Control-Request-Headers", 
            "Content-Type", 
            "Authorization",
            "Accept",         
            "Content-Length"   
        ])
        .allow_methods(vec!["POST", "GET", "OPTIONS", "DELETE", "PUT"])
}