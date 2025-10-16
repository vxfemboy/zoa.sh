use actix_web::{web, App, HttpServer, HttpResponse, Result};
use actix_files::Files;
use tera::Tera;
use std::fs;

mod mods;
mod ascii_art;
mod stars;

use mods::*;
use ascii_art::*;
use stars::*;

// Serve individual cat animation files
async fn cat_action(path: web::Path<String>) -> HttpResponse {
    let action = path.into_inner();
    let content = fs::read_to_string(format!("templates/ascii/cat/{}.txt", action))
        .unwrap_or_else(|_| "Cat animation not found".to_string());
    HttpResponse::Ok().body(content)
}

async fn index(tera: web::Data<Tera>) -> Result<HttpResponse> {
    let nav_items = vec![
        NavItem {
            text: "HOME".to_string(),
            href: "#".to_string(),
        },
        NavItem {
            text: "ABOUT".to_string(),
            href: "#".to_string(),
        },
        NavItem {
            text: "PROJECTS".to_string(),
            href: "#".to_string(),
        },
        NavItem {
            text: "BLOG".to_string(),
            href: "#".to_string(),
        },
        NavItem {
            text: "CONTACT".to_string(),
            href: "#".to_string(),
        },
    ];

    let latest_post = Post {
        title: replace_problematic_chars("Testing Emoji Support 🚀🔥"),
        href: "#".to_string(),
        date: "October 12, 2025".to_string(),
        content: replace_problematic_chars(
            "Testing various emojis: 🌈 rainbow, ⭐ star, 💻 laptop, 🎉 party, ❤️ heart, and 🄯 copyleft!"
        ),
    };

    let categories = [
        "Kernel".to_string(),
        "Security".to_string(),
        "Networking".to_string(),
        "Systems".to_string(),
        "Research".to_string(),
    ];

    let latest_comment = Comment {
        username: "user123".to_string(),
        content: "Love the ASCII aesthetic!".to_string(),
    };

    // Load the ASCII title art from file
    let title_art =
        fs::read_to_string("templates/ascii/title.txt").unwrap_or_else(|_| "ASCII Web".to_string());

    let footer_text = replace_problematic_chars(
        "🄯 222.121.87.in-addr.arpa | Created with Rust, <3 and lots of button presses",
    );

    // Create boxes for different screen sizes
    let make_box_sizes = |title: &str, content: &str| BoxSizes {
        tiny: create_header_box(title, content, 30),
        small: create_header_box(title, content, 40),
        medium: create_header_box(title, content, 60),
        large: create_header_box(title, content, 80),
    };

    let make_footer_sizes = |text: &str| BoxSizes {
        tiny: create_footer_box(text, 30),
        small: create_footer_box(text, 40),
        medium: create_footer_box(text, 60),
        large: create_footer_box(text, 80),
    };

    let stars = generate_stars(150);

    let context = PageContext {
        title_art,
        navigation_box: BoxSizes {
            tiny: create_nav_box(&nav_items, 35),
            small: create_nav_box(&nav_items, 45),
            medium: create_nav_box(&nav_items, 60),
            large: create_nav_box(&nav_items, 80),
        },
        welcome_box: make_box_sizes(
            "WELCOME",
            "Hello and welcome to my website!\n\nThis is a Rust-powered ASCII art website.",
        ),
        latest_post_box: BoxSizes {
            tiny: {
                let wrapped_content = wrap_text(&latest_post.content, 22).join("\n");
                create_header_box(
                    "LATEST POSTS",
                    &format!(
                        "\n{}\n\n{}\n{}\n\n{}\n\nRead more: {}\n\n{}\n",
                        "░".repeat(24),
                        latest_post.title,
                        latest_post.date,
                        wrapped_content,
                        format!("<a href=\"{}\">{}</a>", latest_post.href, latest_post.title),
                        "░".repeat(24)
                    ),
                    30,
                )
            },
            small: {
                let wrapped_content = wrap_text(&latest_post.content, 32).join("\n");
                create_header_box(
                    "LATEST POSTS",
                    &format!(
                        "\n{}\n\n{}\n{}\n\n{}\n\nRead more: {}\n\n{}\n",
                        "░".repeat(34),
                        latest_post.title,
                        latest_post.date,
                        wrapped_content,
                        format!("<a href=\"{}\">{}</a>", latest_post.href, latest_post.title),
                        "░".repeat(34)
                    ),
                    40,
                )
            },
            medium: {
                let wrapped_content = wrap_text(&latest_post.content, 52).join("\n");
                create_header_box(
                    "LATEST POSTS",
                    &format!(
                        "\n{}\n\n{}\n{}\n\n{}\n\nRead more: {}\n\n{}\n",
                        "░".repeat(54),
                        latest_post.title,
                        latest_post.date,
                        wrapped_content,
                        format!("<a href=\"{}\">{}</a>", latest_post.href, latest_post.title),
                        "░".repeat(54)
                    ),
                    60,
                )
            },
            large: {
                // Use max_width - 6 to account for: borders (2) + margins (2) + emoji rounding (2)
                let wrapped_content = wrap_text(&latest_post.content, 72).join("\n");
                create_header_box(
                    "LATEST POSTS",
                    &format!(
                        "\n{}\n\n{}\n{}\n\n{}\n\nRead more: {}\n\n{}\n",
                        "░".repeat(74),
                        latest_post.title,
                        latest_post.date,
                        wrapped_content,
                        format!("<a href=\"{}\">{}</a>", latest_post.href, latest_post.title),
                        "░".repeat(74)
                    ),
                    80,
                )
            },
        },
        about_box: BoxSizes {
            tiny: create_header_box("ABOUT ME", "I press buttons.", 25),
            small: create_header_box("ABOUT ME", "I press buttons.", 30),
            medium: create_header_box("ABOUT ME", "I press buttons.", 35),
            large: create_header_box("ABOUT ME", "I press buttons.", 35),
        },
        categories_box: BoxSizes {
            tiny: create_header_box(
                "CATEGORIES",
                &categories
                    .iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                25,
            ),
            small: create_header_box(
                "CATEGORIES",
                &categories
                    .iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                30,
            ),
            medium: create_header_box(
                "CATEGORIES",
                &categories
                    .iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                35,
            ),
            large: create_header_box(
                "CATEGORIES",
                &categories
                    .iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                35,
            ),
        },
        comments_box: BoxSizes {
            tiny: create_header_box(
                "LATEST COMMENTS",
                &format!(
                    "@{}:\n\"{}\"",
                    latest_comment.username, latest_comment.content
                ),
                25,
            ),
            small: create_header_box(
                "LATEST COMMENTS",
                &format!(
                    "@{}:\n\"{}\"",
                    latest_comment.username, latest_comment.content
                ),
                30,
            ),
            medium: create_header_box(
                "LATEST COMMENTS",
                &format!(
                    "@{}:\n\"{}\"",
                    latest_comment.username, latest_comment.content
                ),
                35,
            ),
            large: create_header_box(
                "LATEST COMMENTS",
                &format!(
                    "@{}:\n\"{}\"",
                    latest_comment.username, latest_comment.content
                ),
                35,
            ),
        },
        footer_box: make_footer_sizes(&footer_text),
        stars,
    };

    let mut tera_context = tera::Context::new();
    tera_context.insert("title_art", &context.title_art);
    tera_context.insert("navigation_box", &context.navigation_box);
    tera_context.insert("welcome_box", &context.welcome_box);
    tera_context.insert("latest_post_box", &context.latest_post_box);
    tera_context.insert("about_box", &context.about_box);
    tera_context.insert("categories_box", &context.categories_box);
    tera_context.insert("comments_box", &context.comments_box);
    tera_context.insert("footer_box", &context.footer_box);
    tera_context.insert("stars", &context.stars);

    let rendered = tera.render("index.html.tera", &tera_context)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize Tera templates
    let tera = Tera::new("templates/**/*").expect("Failed to initialize Tera templates");
    
    println!("Starting server on http://127.0.0.1:8080");
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .route("/", web::get().to(index))
            .route("/cat/{action}", web::get().to(cat_action))
            .service(Files::new("/static", "static"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}


