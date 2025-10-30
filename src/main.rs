use actix::Actor;
use actix_files::Files;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Result};
use std::fs;
use tera::Tera;
use tracing::info;
use tracing::Level;

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
    cache: web::Data<BoxCache>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context()?;

    let template_context = TemplateContextBuilder::new()
        .with_page_context(&context)
        .with_debug(config.debug)
        .build();

    let rendered = tera.render("index.html.tera", &template_context)?;

    // Cache warm: store rendered boxes by keys (basic demonstration)
    let _ = cache.insert(
        "navigation_box_large".to_string(),
        context.navigation_box.large.clone(),
    );
    let _ = cache.insert(
        "welcome_box_large".to_string(),
        context.welcome_box.large.clone(),
    );

    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    // Load configuration first to configure logging
    let config = Config::load().unwrap_or_else(|_| Config::default());

    // Initialize tracing with level controlled by config.debug
    tracing_subscriber::fmt()
        .with_max_level(if config.debug {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .init();

    info!(
        "Starting Web server on {}:{}",
        config.server.host, config.server.port
    );

    // Initialize Tera templates
    let tera = Tera::new("templates/**/*")?;

    // Initialize cache with configuration
    let cache = web::Data::new(BoxCache::with_options(
        config.cache_enabled,
        config.cache_capacity,
    ));

    // Initialize shoutbox server
    let shoutbox_server = ShoutboxServer::default().start();

    let app_config = config.clone();
    HttpServer::new(move || {
        let cfg = app_config.clone();
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .app_data(cache.clone())
            .app_data(web::Data::new(cfg))
            .app_data(web::Data::new(shoutbox_server.clone()))
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
            .route("/ws/shoutbox", web::get().to(shoutbox_ws))
            .route(
                "/api/shoutbox/messages",
                web::get().to(get_shoutbox_messages),
            )
            .route(
                "/api/shoutbox/messages",
                web::post().to(post_shoutbox_message),
            )
            .service(Files::new("/static", "static"))
    })
    .bind(format!("{}:{}", config.server.host, config.server.port))?
    .run()
    .await
    .map_err(|e| AppError::Server(e.to_string()))
}
