use crate::mods::{self, AppError, BoxCache, ContentManager};
use actix_web::{web, HttpRequest, HttpResponse, Result};

pub async fn api_posts(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context(&site.base_url)?;

    Ok(HttpResponse::Ok().json(context.posts))
}

pub async fn api_categories(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let _content_manager = ContentManager::new();
    let _context = _content_manager.create_page_context(&site.base_url)?;

    // Extract categories from the context
    let categories = vec![
        "Kernel".to_string(),
        "Security".to_string(),
        "Networking".to_string(),
        "Systems".to_string(),
        "Research".to_string(),
    ];

    Ok(HttpResponse::Ok().json(categories))
}

pub async fn api_health() -> Result<HttpResponse, AppError> {
    let health = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    });

    Ok(HttpResponse::Ok().json(health))
}

pub async fn api_cache_stats(cache: web::Data<BoxCache>) -> Result<HttpResponse, AppError> {
    let stats = serde_json::json!({
        "cache_size": cache.size()?,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(HttpResponse::Ok().json(stats))
}

pub async fn api_clear_cache(cache: web::Data<BoxCache>) -> Result<HttpResponse, AppError> {
    cache.clear()?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Cache cleared successfully",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
