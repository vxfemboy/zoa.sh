#[macro_use] extern crate rocket;

use rocket::fs::{FileServer, relative};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::fs;
use rand::Rng;

#[derive(Serialize)]
struct BoxSizes {
    tiny: String,   // For very small screens (<300px)
    small: String,  // For small screens (300-600px)
    medium: String, // For medium screens (600-900px)
    large: String,  // For large screens (>900px)
}

#[derive(Serialize)]
struct PageContext {
    title_art: String,
    navigation_box: BoxSizes,
    welcome_box: BoxSizes,
    latest_post_box: BoxSizes,
    about_box: BoxSizes,
    categories_box: BoxSizes,
    comments_box: BoxSizes,
    footer_box: BoxSizes,
    stars: Vec<stars::Star>,
}

#[derive(Serialize)]
struct NavItem {
    text: String,
    href: String,
}

#[derive(Serialize)]
struct Post {
    title: String,
    date: String,
    content: String,
}

#[derive(Serialize)]
struct Comment {
    username: String,
    content: String,
}

pub mod ascii {
    use super::NavItem;

    fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 <= max_width {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }

    fn pad_line(line: &str, width: usize) -> String {
        let line_len = line.chars().count();
        if line_len >= width {
            line.chars().take(width).collect()
        } else {
            format!("{}{}", line, " ".repeat(width - line_len))
        }
    }

    // New function to determine box style based on width
    fn get_box_chars(width: usize) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
        if width < 20 {
            // Minimal style for very narrow screens
            ("+", "-", "+", "|", "|")
        } else if width < 40 {
            // Simple style for narrow screens
            ("┌", "─", "┐", "│", "│")
        } else {
            // Full style for wider screens
            ("╔", "═", "╗", "║", "║")
        }
    }

    pub fn create_footer_box(content: &str, width: usize) -> String {
        let actual_width = width.max(10); // Minimum width of 10 characters
        let content_width = actual_width.saturating_sub(2);
        
        let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 { "+" } else if actual_width < 40 { "└" } else { "╚" };
        let bottom_right = if actual_width < 20 { "+" } else if actual_width < 40 { "┘" } else { "╝" };
        
        let top = format!("{}{}{}\n", top_left, horizontal.repeat(content_width), top_right);
        let bottom = format!("{}{}{}\n", bottom_left, horizontal.repeat(content_width), bottom_right);
        
        let text_len = content.chars().count();
        let middle = if text_len >= content_width {
            let truncated: String = content.chars().take(content_width.saturating_sub(4)).collect();
            format!("{} {}... {}\n", left_border, truncated, right_border)
        } else {
            let padding = (content_width.saturating_sub(text_len)) / 2;
            let extra_space = content_width.saturating_sub(padding.saturating_mul(2)).saturating_sub(text_len);
            format!("{} {}{}{}{}\n", 
                left_border,
                " ".repeat(padding),
                content,
                " ".repeat(extra_space),
                right_border
            )
        };
        
        format!("{}{}{}", top, middle, bottom)
    }

    pub fn create_box(content: &str, width: usize) -> String {
        let actual_width = width.max(10); // Minimum width of 10 characters
        let content_width = actual_width.saturating_sub(2);
        
        let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 { "+" } else if actual_width < 40 { "└" } else { "╚" };
        let bottom_right = if actual_width < 20 { "+" } else if actual_width < 40 { "┘" } else { "╝" };
        
        let top = format!("{}{}{}\n", top_left, horizontal.repeat(content_width), top_right);
        let bottom = format!("{}{}{}\n", bottom_left, horizontal.repeat(content_width), bottom_right);
        
        let wrapped_lines = wrap_text(content, content_width);
        let middle = wrapped_lines.iter()
            .map(|line| format!("{}{}{}\n", 
                left_border,
                pad_line(line, content_width),
                right_border
            ))
            .collect::<String>();
        
        format!("{}{}{}", top, middle, bottom)
    }

    pub fn create_header_box(title: &str, content: &str, width: usize) -> String {
        let actual_width = width.max(10); // Minimum width of 10 characters
        let content_width = actual_width.saturating_sub(2);
        
        let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 { "+" } else if actual_width < 40 { "└" } else { "╚" };
        let bottom_right = if actual_width < 20 { "+" } else if actual_width < 40 { "┘" } else { "╝" };
        let header_sep_left = if actual_width < 20 { "+" } else if actual_width < 40 { "├" } else { "╠" };
        let header_sep_right = if actual_width < 20 { "+" } else if actual_width < 40 { "┤" } else { "╣" };
        
        let top = format!("{}{}{}\n", top_left, horizontal.repeat(content_width), top_right);
        let header_sep = format!("{}{}{}\n", header_sep_left, horizontal.repeat(content_width), header_sep_right);
        let bottom = format!("{}{}{}\n", bottom_left, horizontal.repeat(content_width), bottom_right);
        
        // Center the title
        let title_len = title.chars().count();
        let padding = (content_width.saturating_sub(title_len)) / 2;
        let extra_space = if (content_width.saturating_sub(title_len)) % 2 != 0 { 1 } else { 0 };
        let title_line = format!("{}{}{}{}{}\n", 
            left_border,
            " ".repeat(padding),
            title,
            " ".repeat(padding + extra_space),
            right_border
        );

        // Split content into lines and pad each line
        let content_lines = content.lines()
            .map(|line| format!("{} {}{} {}\n", 
                left_border,
                line,
                " ".repeat(content_width.saturating_sub(line.chars().count()).saturating_sub(2)),
                right_border
            ))
            .collect::<String>();
        
        format!("{}{}{}{}{}", top, title_line, header_sep, content_lines, bottom)
    }

    pub fn create_nav_box(items: &[NavItem], width: usize) -> String {
        let actual_width = width.max(10); // Minimum width of 10 characters
        let content_width = actual_width.saturating_sub(2);
        
        let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 { "+" } else if actual_width < 40 { "└" } else { "╚" };
        let bottom_right = if actual_width < 20 { "+" } else if actual_width < 40 { "┘" } else { "╝" };
        
        let nav_items = items.iter()
            .map(|item| format!("<a href=\"{}\">{}</a>", item.href, item.text))
            .collect::<Vec<_>>();
        
        let nav_content = nav_items.join(" | ");

        let top = format!("{}{}{}\n", top_left, horizontal.repeat(content_width), top_right);
        let bottom = format!("{}{}{}\n", bottom_left, horizontal.repeat(content_width), bottom_right);
        
        // Center the navigation items
        let line_len = nav_content.chars().count();
        let padding = (content_width.saturating_sub(line_len)) / 2;
        let extra_space = content_width.saturating_sub(padding * 2 + line_len);
        
        let content_line = format!("{}{}{}{}{}",
            left_border,
            " ".repeat(padding),
            nav_content,
            " ".repeat(padding + extra_space),
            right_border
        );
        
        format!("{}{}\n{}", top, content_line, bottom)
    }
}

pub mod stars {
    use rand::Rng;
    use serde::Serialize;

    const STAR_CHARS: &[&str] = &[
        "✦", "✧", "★", "☆", "✯", "✡",
        "✵", "❋", "❆", "❉", "✺", "✹",
        "✸", "✶", "✷", "✵", "✴", "✳",
        "*", ".", "⋆", "+", "˚"
    ];

    #[derive(Serialize)]
    pub struct Star {
        pub x: i32,
        pub y: i32,
        pub char: String,
        pub delay: f32,
        pub size: f32,
    }

    pub fn generate_stars(count: usize) -> Vec<Star> {
        let mut rng = rand::thread_rng();
        let mut stars = Vec::with_capacity(count);
        
        for _ in 0..count {
            let star = Star {
                x: rng.gen_range(0..100), // Percentage of screen width
                y: rng.gen_range(0..100), // Percentage of screen height
                char: STAR_CHARS[rng.gen_range(0..STAR_CHARS.len())].to_string(),
                delay: rng.gen_range(0.0..5.0), // Random animation delay up to 5 seconds
                size: rng.gen_range(0.8..2.0), // Random size multiplier
            };
            stars.push(star);
        }
        
        stars
    }
}

#[get("/")]
fn index() -> Template {
    let nav_items = vec![
        NavItem { text: "HOME".to_string(), href: "#".to_string() },
        NavItem { text: "ABOUT".to_string(), href: "#".to_string() },
        NavItem { text: "PROJECTS".to_string(), href: "#".to_string() },
        NavItem { text: "BLOG".to_string(), href: "#".to_string() },
        NavItem { text: "CONTACT".to_string(), href: "#".to_string() },
    ];

    let latest_post = Post {
        title: "Anti-Forensics in the Kernel".to_string(),
        date: "May 4, 2025".to_string(),
        content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit.".to_string(),
    };

    let categories = vec![
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
    let title_art = fs::read_to_string("templates/ascii/title.txt")
        .unwrap_or_else(|_| "ASCII Web".to_string());

    let footer_text = "🄯 222.121.87.in-addr.arpa | Created with Rust, <3 and lots of button presses";

    // Create boxes for different screen sizes
    let make_box_sizes = |title: &str, content: &str| BoxSizes {
        tiny: ascii::create_header_box(title, content, 30),
        small: ascii::create_header_box(title, content, 40),
        medium: ascii::create_header_box(title, content, 60),
        large: ascii::create_header_box(title, content, 80),
    };

    let make_nav_sizes = |items: &[NavItem]| BoxSizes {
        tiny: ascii::create_nav_box(items, 35),
        small: ascii::create_nav_box(items, 45),
        medium: ascii::create_nav_box(items, 60),
        large: ascii::create_nav_box(items, 80),
    };

    let make_footer_sizes = |text: &str| BoxSizes {
        tiny: ascii::create_footer_box(text, 30),
        small: ascii::create_footer_box(text, 40),
        medium: ascii::create_footer_box(text, 60),
        large: ascii::create_footer_box(text, 80),
    };

    let stars = stars::generate_stars(150);

    let context = PageContext {
        title_art,
        navigation_box: BoxSizes {
            tiny: ascii::create_nav_box(&nav_items, 35),
            small: ascii::create_nav_box(&nav_items, 45),
            medium: ascii::create_nav_box(&nav_items, 60),
            large: ascii::create_nav_box(&nav_items, 80),
        },
        welcome_box: make_box_sizes(
            "WELCOME",
            "Hello and welcome to my website!\n\nThis is a Rust-powered ASCII art website."
        ),
        latest_post_box: make_box_sizes(
            "LATEST POSTS",
            &format!("\n{}\n\n{}\n{}\n\n{}\n\nRead more: {}\n\n{}\n",
                "░".repeat(60),
                latest_post.title,
                latest_post.date,
                latest_post.content,
                format!("<a href=\"#\">{}</a>", latest_post.title),
                "░".repeat(60)
            ).lines()
            .map(|line| {
                let line_len = line.chars().count();
                if line_len > 58 {  // Max width minus some padding
                    format!("{:.58}", line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
        ),
        about_box: BoxSizes {
            tiny: ascii::create_header_box("ABOUT ME", "I press buttons.", 25),
            small: ascii::create_header_box("ABOUT ME", "I press buttons.", 30),
            medium: ascii::create_header_box("ABOUT ME", "I press buttons.", 35),
            large: ascii::create_header_box("ABOUT ME", "I press buttons.", 35),
        },
        categories_box: BoxSizes {
            tiny: ascii::create_header_box(
                "CATEGORIES",
                &categories.iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                25
            ),
            small: ascii::create_header_box(
                "CATEGORIES",
                &categories.iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                30
            ),
            medium: ascii::create_header_box(
                "CATEGORIES",
                &categories.iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                35
            ),
            large: ascii::create_header_box(
                "CATEGORIES",
                &categories.iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                35
            ),
        },
        comments_box: BoxSizes {
            tiny: ascii::create_header_box(
                "LATEST COMMENTS",
                &format!("@{}:\n\"{}\"", latest_comment.username, latest_comment.content),
                25
            ),
            small: ascii::create_header_box(
                "LATEST COMMENTS",
                &format!("@{}:\n\"{}\"", latest_comment.username, latest_comment.content),
                30
            ),
            medium: ascii::create_header_box(
                "LATEST COMMENTS",
                &format!("@{}:\n\"{}\"", latest_comment.username, latest_comment.content),
                35
            ),
            large: ascii::create_header_box(
                "LATEST COMMENTS",
                &format!("@{}:\n\"{}\"", latest_comment.username, latest_comment.content),
                35
            ),
        },
        footer_box: make_footer_sizes(footer_text),
        stars,
    };

    Template::render("index", context)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/static", FileServer::from(relative!("static")))
        .attach(Template::fairing())
} 