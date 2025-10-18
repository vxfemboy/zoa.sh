use actix_files::Files;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Result};
use std::fs;
use tera::Tera;
use tracing::{error, info};

mod mods;

use mods::*;

// Serve individual cat animation files
async fn cat_action(path: web::Path<String>) -> Result<HttpResponse, AppError> {
    let action = path.into_inner();
    let content =
        fs::read_to_string(format!("templates/ascii/cat/{}.txt", action)).map_err(|e| {
            AppError::FileNotFound(format!("Cat animation '{}' not found: {}", action, e))
        })?;
    Ok(HttpResponse::Ok().body(content))
}

async fn index(
    tera: web::Data<Tera>,
    _cache: web::Data<BoxCache>,
) -> Result<HttpResponse, AppError> {
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context()?;

    let template_context = TemplateContextBuilder::new()
        .with_page_context(&context)
        .build();

    let rendered = tera.render("index.html.tera", &template_context)?;

    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        error!("Failed to load config: {}. Using defaults.", e);
        Config::default()
    });

    info!(
        "Starting ASCII Web server on {}:{}",
        config.server.host, config.server.port
    );

    // Initialize Tera templates
    let tera = Tera::new("templates/**/*")?;

    // Initialize cache
    let cache = web::Data::new(BoxCache::new());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .app_data(cache.clone())
            .wrap(middleware::Logger::default())
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-XSS-Protection", "1; mode=block")),
            )
            .route("/", web::get().to(index))
            .route("/cat/{action}", web::get().to(cat_action))
            .route("/api/posts", web::get().to(api_posts))
            .route("/api/categories", web::get().to(api_categories))
            .route("/api/health", web::get().to(api_health))
            .route("/api/cache/stats", web::get().to(api_cache_stats))
            .route("/api/cache/clear", web::post().to(api_clear_cache))
            .service(Files::new("/static", "static"))
    })
    .bind(format!("{}:{}", config.server.host, config.server.port))?
    .run()
    .await
    .map_err(|e| AppError::Server(e.to_string()))
}
