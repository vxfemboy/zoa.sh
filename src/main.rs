use actix_files::Files;
use actix_web::middleware::from_fn;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use tera::Tera;
use tracing::info;
use tracing::Level;

mod mods;

use mods::constants::*;
use mods::markdown::load_all_posts;
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

/// Query parameters for blog filtering
#[derive(Deserialize)]
pub struct BlogQuery {
    tag: Option<String>,
}

/// Blog listing page - shows all posts, optionally filtered by tag
async fn blog_index(
    req: HttpRequest,
    tera: web::Data<Tera>,
    config: web::Data<Config>,
    query: web::Query<BlogQuery>,
) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context(&site.base_url)?;

    // Extract all unique tags from posts
    let all_tags: Vec<String> = {
        let mut tags_set: HashSet<String> = HashSet::new();
        for post in &context.posts {
            for tag in &post.tags {
                tags_set.insert(tag.clone());
            }
        }
        let mut tags: Vec<String> = tags_set.into_iter().collect();
        tags.sort();
        tags
    };

    // Validate and filter by tag - only allow known tags (prevents XSS)
    let valid_tag = query.tag.as_ref().filter(|t| all_tags.contains(t));

    // Filter posts by tag if specified and valid
    let filtered_posts: Vec<_> = if let Some(tag) = valid_tag {
        context
            .posts
            .iter()
            .filter(|p| p.tags.contains(tag))
            .collect()
    } else {
        context.posts.iter().collect()
    };

    // Generate blog header box (tag is guaranteed safe since it came from our posts)
    let header_text = if let Some(ref tag) = valid_tag {
        format!("Posts tagged: {}", tag)
    } else {
        "All posts from the blog".to_string()
    };
    let blog_header = ResponsiveBoxes::new_header("BLOG POSTS", &header_text);

    // Generate categories box from tags (bullet point style like original)
    let categories_box = ResponsiveBoxes::new_tag_categories(&all_tags, valid_tag);

    // Generate boxes for each post - truncate and wrap content to fit within boxes
    let posts_with_boxes: Vec<serde_json::Value> = filtered_posts
        .iter()
        .map(|post| {
            // Truncate then wrap for each box size
            let content_tiny = format!(
                "{}\n\n{}\n\n<a href=\"{}\">[ Read >> ]</a>",
                post.date,
                wrap_text(&truncate_text(&post.content, 60), WRAP_WIDTH_TINY).join("\n"),
                post.href
            );
            let content_small = format!(
                "{}\n\n{}\n\n<a href=\"{}\">[ Read more >> ]</a>",
                post.date,
                wrap_text(&truncate_text(&post.content, 80), WRAP_WIDTH_SMALL).join("\n"),
                post.href
            );
            let content_medium = format!(
                "{}\n\n{}\n\n<a href=\"{}\">[ Read more >> ]</a>",
                post.date,
                wrap_text(&truncate_text(&post.content, 120), WRAP_WIDTH_MEDIUM).join("\n"),
                post.href
            );
            let content_large = format!(
                "{}\n\n{}\n\n<a href=\"{}\">[ Read more >> ]</a>",
                post.date,
                wrap_text(&truncate_text(&post.content, 200), WRAP_WIDTH_LARGE).join("\n"),
                post.href
            );
            serde_json::json!({
                "title": post.title,
                "date": post.date,
                "href": post.href,
                "box_tiny": create_header_box(&post.title, &content_tiny, BOX_WIDTH_TINY),
                "box_small": create_header_box(&post.title, &content_small, BOX_WIDTH_SMALL),
                "box_medium": create_header_box(&post.title, &content_medium, BOX_WIDTH_MEDIUM),
                "box_large": create_header_box(&post.title, &content_large, BOX_WIDTH_LARGE),
            })
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("title_art", &context.title_art);
    ctx.insert("navigation_box", &context.navigation_box);
    // RSS box
    let rss_box = ResponsiveBoxes::new_rss_box();
    ctx.insert(
        "rss_box",
        &BoxSizes {
            tiny: rss_box.tiny,
            small: rss_box.small,
            medium: rss_box.medium,
            large: rss_box.large,
        },
    );
    ctx.insert(
        "categories_box",
        &BoxSizes {
            tiny: categories_box.tiny,
            small: categories_box.small,
            medium: categories_box.medium,
            large: categories_box.large,
        },
    );
    ctx.insert("footer_box", &context.footer_box);
    ctx.insert("stars", &context.stars);
    ctx.insert("posts", &posts_with_boxes);
    ctx.insert(
        "blog_header_box",
        &BoxSizes {
            tiny: blog_header.tiny,
            small: blog_header.small,
            medium: blog_header.medium,
            large: blog_header.large,
        },
    );
    ctx.insert("current_tag", &valid_tag);
    ctx.insert("debug", &config.debug);

    let rendered = tera.render("blog.html.tera", &ctx)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

/// RSS feed for blog posts
async fn rss_feed() -> Result<HttpResponse, AppError> {
    let posts = load_all_posts("posts");

    let items: String = posts
        .iter()
        .map(|post| {
            let description = truncate_text(&post.content_plain, 300);
            format!(
                r#"    <item>
      <title>{}</title>
      <link>https://zoa.sh/post/{}</link>
      <guid>https://zoa.sh/post/{}</guid>
      <pubDate>{}</pubDate>
      <description><![CDATA[{}]]></description>
      <content:encoded><![CDATA[{}]]></content:encoded>
    </item>"#,
                xml_escape(&post.title),
                post.slug,
                post.slug,
                post.date,
                description,
                post.content_html
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:content="http://purl.org/rss/1.0/modules/content/">
  <channel>
    <title>vxfemboy blog</title>
    <link>https://zoa.sh/blog</link>
    <description>Blog posts from vxfemboy</description>
    <language>en-us</language>
    <atom:link href="https://zoa.sh/rss.xml" rel="self" type="application/rss+xml"/>
{}
  </channel>
</rss>"#,
        items
    );

    Ok(HttpResponse::Ok()
        .content_type("application/rss+xml; charset=utf-8")
        .body(rss))
}

/// Robots.txt for SEO
async fn robots_txt() -> HttpResponse {
    let robots = r#"User-agent: *
Allow: /

Sitemap: https://zoa.sh/sitemap.xml
"#;
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(robots)
}

/// Sitemap for SEO
async fn sitemap() -> Result<HttpResponse, AppError> {
    let posts = load_all_posts("posts");

    let post_urls: String = posts
        .iter()
        .map(|post| {
            format!(
                r#"  <url>
    <loc>https://zoa.sh/post/{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>"#,
                post.slug, post.date
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sitemap = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://zoa.sh/</loc>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://zoa.sh/about</loc>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://zoa.sh/projects</loc>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://zoa.sh/uses</loc>
    <changefreq>monthly</changefreq>
    <priority>0.6</priority>
  </url>
  <url>
    <loc>https://zoa.sh/now</loc>
    <changefreq>weekly</changefreq>
    <priority>0.7</priority>
  </url>
  <url>
    <loc>https://zoa.sh/blog</loc>
    <changefreq>daily</changefreq>
    <priority>0.9</priority>
  </url>
{}
</urlset>"#,
        post_urls
    );

    Ok(HttpResponse::Ok()
        .content_type("application/xml; charset=utf-8")
        .body(sitemap))
}

/// Escape special XML characters
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Truncate text to max length at word boundary
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let truncated = &text[..max_len];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &truncated[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

/// Individual post page
async fn post_view(
    req: HttpRequest,
    path: web::Path<String>,
    tera: web::Data<Tera>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let slug = path.into_inner();
    let posts = load_all_posts("posts");

    let post = posts
        .into_iter()
        .find(|p| p.slug == slug)
        .ok_or_else(|| AppError::FileNotFound(format!("Post '{}' not found", slug)))?;

    // Generate combined post header box with title, date, divider, and back link
    let post_header = ResponsiveBoxes::new_post_header(&post.title, &post.date);

    let mut ctx = tera::Context::new();
    ctx.insert("title", &post.title);
    ctx.insert("date", &post.date);
    ctx.insert("slug", &post.slug);
    ctx.insert("content_html", &post.content_html);
    ctx.insert("debug", &config.debug);

    // Post header box (combined with back link)
    ctx.insert(
        "post_header_box",
        &BoxSizes {
            tiny: post_header.tiny,
            small: post_header.small,
            medium: post_header.medium,
            large: post_header.large,
        },
    );

    // Load shared layout elements
    let content_manager = ContentManager::new();
    let page_context = content_manager.create_page_context(&site.base_url)?;
    ctx.insert("title_art", &page_context.title_art);
    ctx.insert("navigation_box", &page_context.navigation_box);
    ctx.insert("footer_box", &page_context.footer_box);
    ctx.insert("stars", &page_context.stars);

    let rendered = tera.render("post.html.tera", &ctx)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

/// About page - Zoa's bio, experience, skills, education, and links.
async fn about(
    req: HttpRequest,
    tera: web::Data<Tera>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context(&site.base_url)?;
    let about = mods::about::build_about_boxes(&site);

    let mut ctx = tera::Context::new();
    // Shared chrome reused from the main page context.
    ctx.insert("title_art", &context.title_art);
    ctx.insert("navigation_box", &context.navigation_box);
    ctx.insert("footer_box", &context.footer_box);
    ctx.insert("stars", &context.stars);
    // ASCII portrait (GitHub avatar) for the sidebar.
    ctx.insert("profile_art", &mods::about::load_profile_art());
    // About-specific sections.
    ctx.insert("summary_box", &about.summary_box);
    // Wide desktop variant + compact mobile variant (swapped by CSS media query).
    ctx.insert("experience_box", &mods::about::experience_box(78, false));
    ctx.insert(
        "experience_box_mobile",
        &mods::about::experience_box(40, true),
    );
    ctx.insert("skills_box", &about.skills_box);
    ctx.insert("links_box", &about.links_box);
    ctx.insert("debug", &config.debug);

    let rendered = tera.render("about.html.tera", &ctx)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

async fn index(
    req: HttpRequest,
    tera: web::Data<Tera>,
    cache: web::Data<BoxCache>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context(&site.base_url)?;

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

/// Projects page — portfolio cards.
async fn projects(
    req: HttpRequest,
    tera: web::Data<Tera>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let context = ContentManager::new().create_page_context(&site.base_url)?;

    let mut ctx = tera::Context::new();
    ctx.insert("title_art", &context.title_art);
    ctx.insert("navigation_box", &context.navigation_box);
    ctx.insert("footer_box", &context.footer_box);
    ctx.insert("stars", &context.stars);
    ctx.insert("projects", &mods::projects::project_boxes(&site));
    ctx.insert("debug", &config.debug);

    let rendered = tera.render("projects.html.tera", &ctx)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

/// Render a simple centered-card page (uses / now): shared chrome + a list of
/// boxes under `key`, rendered with `template`.
async fn card_page(
    tera: &web::Data<Tera>,
    config: &web::Data<Config>,
    template: &str,
    key: &str,
    boxes: Vec<BoxSizes>,
    base_url: &str,
) -> Result<HttpResponse, AppError> {
    let context = ContentManager::new().create_page_context(base_url)?;
    let mut ctx = tera::Context::new();
    ctx.insert("title_art", &context.title_art);
    ctx.insert("navigation_box", &context.navigation_box);
    ctx.insert("footer_box", &context.footer_box);
    ctx.insert("stars", &context.stars);
    ctx.insert(key, &boxes);
    ctx.insert("debug", &config.debug);
    let rendered = tera.render(template, &ctx)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

/// Uses page — tools & setup (uses.tech style).
async fn uses(
    req: HttpRequest,
    tera: web::Data<Tera>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    card_page(
        &tera,
        &config,
        "uses.html.tera",
        "uses",
        mods::uses::uses_boxes(),
        &site.base_url,
    )
    .await
}

/// Now page — current focus (nownownow.com style).
async fn now(
    req: HttpRequest,
    tera: web::Data<Tera>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    card_page(
        &tera,
        &config,
        "now.html.tera",
        "now",
        mods::now::now_boxes(&site),
        &site.base_url,
    )
    .await
}

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    // Load configuration first to configure logging
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("CONFIG LOAD ERROR: {e}");
            Config::default()
        }
    };

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

    // Auto-refresh the /about portrait from the configured source (e.g. the
    // GitHub avatar). Runs off-thread so a slow/offline fetch doesn't block
    // startup; on failure the committed templates/ascii/avatar.* is kept.
    //
    // Skip if the portrait was regenerated within `refresh_interval_secs` — this
    // both rate-limits GitHub and prevents a restart loop when a file watcher
    // (dev.sh) is watching templates/ (the refresh writes avatar.html there).
    let pfp_fresh = std::fs::metadata("templates/ascii/avatar.html")
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|age| age.as_secs() < config.pfp.refresh_interval_secs)
        .unwrap_or(false);
    if config.pfp.auto_update && !pfp_fresh {
        let pfp = config.pfp.clone();
        match tokio::task::spawn_blocking(move || {
            let opts = zoa_sh::ansi_image::PfpOptions {
                width_cells: pfp.width,
                brightness: pfp.brightness as f32,
                contrast: pfp.contrast as f32,
            };
            zoa_sh::ansi_image::refresh(
                &pfp.url,
                &opts,
                "templates/ascii/avatar.html",
                "templates/ascii/avatar.ans",
            )
        })
        .await
        {
            Ok(Ok(())) => info!("pfp: refreshed portrait from {}", config.pfp.url),
            Ok(Err(e)) => {
                tracing::warn!("pfp: refresh failed ({e}); keeping committed avatar")
            }
            Err(e) => tracing::warn!("pfp: refresh task panicked ({e}); keeping committed avatar"),
        }
    }

    // Initialize Tera templates
    let tera = Tera::new("templates/**/*")?;

    // Initialize cache with configuration
    let cache = web::Data::new(BoxCache::with_options(
        config.cache_enabled,
        config.cache_capacity,
    ));

    let app_config = config.clone();
    HttpServer::new(move || {
        let cfg = app_config.clone();
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .app_data(cache.clone())
            .app_data(web::Data::new(cfg))
            .wrap(middleware::Logger::default())
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-XSS-Protection", "1; mode=block")),
            )
            // Outermost: rewrite HTML → ANSI for curl/wget clients (all pages).
            .wrap(from_fn(mods::text::curl_ansi))
            .route("/", web::get().to(index))
            .route("/about", web::get().to(about))
            .route("/projects", web::get().to(projects))
            .route("/uses", web::get().to(uses))
            .route("/now", web::get().to(now))
            .route("/blog", web::get().to(blog_index))
            .route("/post/{slug}", web::get().to(post_view))
            .route("/rss.xml", web::get().to(rss_feed))
            .route("/feed", web::get().to(rss_feed))
            .route("/sitemap.xml", web::get().to(sitemap))
            .route("/robots.txt", web::get().to(robots_txt))
            .route("/cat/{action}", web::get().to(cat_action))
            .route("/api/posts", web::get().to(api_posts))
            .route("/api/categories", web::get().to(api_categories))
            .route("/api/health", web::get().to(api_health))
            .route("/api/cache/stats", web::get().to(api_cache_stats))
            .route("/api/cache/clear", web::post().to(api_clear_cache))
            // Serve the WASM with no-cache so a browser never mixes a stale
            // zoa_sh.js with a freshly rebuilt zoa_sh_bg.wasm (or vice versa) —
            // that hash mismatch throws "index out of bounds" at load. Must be
            // registered before the general /static handler so it matches first.
            .service(
                web::scope("/static/wasm")
                    .wrap(middleware::DefaultHeaders::new().add(("Cache-Control", "no-cache")))
                    .service(Files::new("", "static/wasm")),
            )
            .service(Files::new("/static", "static"))
            // Per-post image assets: a post at /post/<slug> can reference
            // `assets/<slug>/1.png`, which the browser resolves to
            // /post/assets/<slug>/1.png. Served from posts/assets/.
            .service(Files::new("/post/assets", "posts/assets"))
    })
    .bind(format!("{}:{}", config.server.host, config.server.port))?
    .run()
    .await
    .map_err(|e| AppError::Server(e.to_string()))
}
