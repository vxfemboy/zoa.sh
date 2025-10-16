#[macro_use]
extern crate rocket;
extern crate rand;
//use rand::RngCore;
use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::fs;
extern crate unicode_width;

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
pub struct NavItem {
    text: String,
    href: String,
}

#[derive(Serialize)]
struct Post {
    title: String,
    date: String,
    content: String,
    href: String,
}

#[derive(Serialize)]
struct Comment {
    username: String,
    content: String,
}

pub mod ascii {
    use super::NavItem;
    use unicode_width::UnicodeWidthChar;

    // ═══════════════════════════════════════════════════════════════════════════
    // Unicode & Emoji Handling Strategy for ASCII Art
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // The Challenge:
    // - Unicode characters (especially emoji) render at different widths across browsers/OS
    // - CSS `unicode-width` reports standard widths, but actual rendering varies by font
    // - There is NO perfect server-side solution (see: https://nolanlawson.com/2022/04/08/)
    //
    // Our Solution (Web Search Best Practices):
    // 1. ✅ Use monospaced font stack (see templates/index.html.tera)
    // 2. ✅ Use unicode-width crate for 99% of characters (automatic)
    // 3. ✅ Manual overrides for problematic characters (get_actual_char_width)
    // 4. ✅ Support fractional widths (1.5, 1.3, etc.)
    // 5. ✅ Skip HTML tags but count rendered elements like <img>
    //
    // How to Add New Emoji:
    // 1. Add it to your content (e.g., footer, blog post)
    // 2. Build and test in YOUR browser
    // 3. If borders misalign, add override in get_actual_char_width()
    // 4. Adjust value (try: 1.0, 1.3, 1.5, 1.8, 2.0) until perfect
    //
    // Note: Different browsers may need different values. Optimize for your main audience.
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let visual_width = count_visual_width_excluding_html(&test_line);

            if visual_width <= max_width {
                current_line = test_line;
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
        let visual_width = count_visual_width_excluding_html(line);
        if visual_width >= width {
            line.to_string()
        } else {
            format!("{}{}", line, " ".repeat(width - visual_width))
        }
    }

    pub fn replace_problematic_chars(text: &str) -> String {
        // Automatically replace emojis and special Unicode chars that don't render
        // consistently in monospace fonts

        let mut result = String::new();

        for c in text.chars() {
            let codepoint = c as u32;

            // Check if character is problematic (emoji or special Unicode)
            let is_problematic = matches!(codepoint,
                // Emoji ranges (consolidated to avoid overlaps)
                0x1F100..=0x1F9FF |  // All emoji blocks: Enclosed Alphanumeric Supplement through Supplemental Symbols
                0x2600..=0x27BF |    // Misc symbols + Dingbats (consolidated)
                0x2B00..=0x2BFF |    // Miscellaneous Symbols and Arrows (includes ⭐)
                0x2460..=0x24FF |    // Enclosed Alphanumerics
                0xFE0F |             // Variation Selector-16
                0x200D |             // Zero Width Joiner
                0x200B..=0x200F |    // Zero-width spaces
                0xFEFF               // Zero Width No-Break Space
            );

            if is_problematic {
                // Handle specific symbols
                match c {
                    // Zero-width characters - remove completely
                    '\u{fe0f}' | '\u{200d}' | '\u{200b}' | '\u{200c}' | '\u{200f}' | '\u{feff}' => {
                    }

                    // Special: Use custom copyleft SVG
                    '🄯' => {
                        result.push_str("<img src=\"/static/copyleft.svg\" alt=\"copyleft\" class=\"emoji-img\">");
                    }

                    // All other emojis/special chars: convert to SVG using Twemoji
                    // Each SVG is styled to take exactly 1 monospace character width
                    _ => {
                        // Convert codepoint to hex string for Twemoji CDN
                        let hex_code = format!("{:x}", codepoint);
                        result.push_str(&format!(
                            "<img src=\"https://cdn.jsdelivr.net/gh/twitter/twemoji@latest/assets/svg/{}.svg\" alt=\"emoji\" class=\"emoji-img\">",
                            hex_code
                        ));
                    }
                }
            } else {
                // Normal character - keep as is
                result.push(c);
            }
        }

        result
    }

    fn get_actual_char_width(c: char) -> f32 {
        // For characters that still need width adjustments
        match c {
            '©' => 1.0, // Copyright
            '❤' => 1.0, // Heart (without variation selector)
            // All zero-width characters
            '\u{fe0f}' => 0.0, // Variation Selector-16
            '\u{200d}' => 0.0, // Zero Width Joiner
            '\u{200b}' => 0.0, // Zero Width Space
            '\u{200c}' => 0.0, // Zero Width Non-Joiner
            '\u{200f}' => 0.0, // Right-to-Left Mark
            '\u{feff}' => 0.0, // Zero Width No-Break Space
            _ => c.width().unwrap_or(1) as f32,
        }
    }

    fn count_visual_width_excluding_html(text: &str) -> usize {
        let mut visual_width: f32 = 0.0;
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '<' {
                // Check if this looks like an HTML tag (letter or / after <)
                if i + 1 < chars.len() && (chars[i + 1].is_alphabetic() || chars[i + 1] == '/') {
                    // Check if it's an <img> tag (which renders and takes up space)
                    let is_img_tag = i + 3 < chars.len()
                        && chars[i + 1] == 'i'
                        && chars[i + 2] == 'm'
                        && chars[i + 3] == 'g';

                    // Skip the entire HTML tag until we find the closing >
                    i += 1; // Skip the <
                    while i < chars.len() && chars[i] != '>' {
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1; // Skip the >
                    }

                    // If it was an <img> tag, count it as exactly 1 character width
                    // CSS forces width:1ch so this should be exact
                    if is_img_tag {
                        visual_width += 1.0; // Exactly 1 character
                    }
                } else {
                    // Not an HTML tag, count its visual width
                    visual_width += get_actual_char_width(chars[i]);
                    i += 1;
                }
            } else {
                // Regular character - use actual width (with overrides for problematic chars)
                visual_width += get_actual_char_width(chars[i]);
                i += 1;
            }
        }

        // Round to nearest integer for space padding
        visual_width.round() as usize
    }

    fn create_content_line(
        left_border: &str,
        content: &str,
        right_border: &str,
        target_width: usize,
        center: bool,
    ) -> String {
        // Count VISUAL WIDTH excluding HTML tags
        // (so <a href="#">TEXT</a> with emoji counts properly: TEXT = 4, 🄯 = 2, etc.)
        let content_visual_width = count_visual_width_excluding_html(content);

        if content_visual_width >= target_width {
            // Content is too long
            format!("{}{}{}\n", left_border, content, right_border)
        } else {
            // Pad with spaces to reach exact visual width
            let spaces_needed = target_width - content_visual_width;

            let line_content = if center {
                let left_spaces = spaces_needed / 2;
                let right_spaces = spaces_needed - left_spaces;
                format!(
                    "{}{}{}",
                    " ".repeat(left_spaces),
                    content,
                    " ".repeat(right_spaces)
                )
            } else {
                format!("{}{}", content, " ".repeat(spaces_needed))
            };

            format!("{}{}{}\n", left_border, line_content, right_border)
        }
    }

    // New function to determine box style based on width
    fn get_box_chars(
        width: usize,
    ) -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
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

        // Content width = total width minus 2 (for left and right borders)
        // This ensures the horizontal line and content have the same width
        let content_width = actual_width - 2;

        let (top_left, horizontal, top_right, left_border, right_border) =
            get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "└"
        } else {
            "╚"
        };
        let bottom_right = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "┘"
        } else {
            "╝"
        };

        // The horizontal line is exactly content_width characters
        let horizontal_line = horizontal.repeat(content_width);

        let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
        let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

        // Calculate the actual VISUAL WIDTH of the horizontal line and match it
        let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);
        let middle = create_content_line(
            left_border,
            content,
            right_border,
            horizontal_visual_width,
            true,
        );

        format!("{}{}{}", top, middle, bottom)
    }

    pub fn create_box(content: &str, width: usize) -> String {
        let actual_width = width.max(10); // Minimum width of 10 characters
        let content_width = actual_width.saturating_sub(2);

        let (top_left, horizontal, top_right, left_border, right_border) =
            get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "└"
        } else {
            "╚"
        };
        let bottom_right = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "┘"
        } else {
            "╝"
        };

        let horizontal_line = horizontal.repeat(content_width);
        let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
        let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

        // Calculate visual width of horizontal line
        let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);
        let wrapped_lines = wrap_text(content, horizontal_visual_width);
        let middle = wrapped_lines
            .iter()
            .map(|line| {
                format!(
                    "{}{}{}\n",
                    left_border,
                    pad_line(line, horizontal_visual_width),
                    right_border
                )
            })
            .collect::<String>();

        format!("{}{}{}", top, middle, bottom)
    }

    pub fn create_header_box(title: &str, content: &str, width: usize) -> String {
        let actual_width = width.max(10); // Minimum width of 10 characters
        let content_width = actual_width.saturating_sub(2);

        let (top_left, horizontal, top_right, left_border, right_border) =
            get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "└"
        } else {
            "╚"
        };
        let bottom_right = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "┘"
        } else {
            "╝"
        };
        let header_sep_left = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "├"
        } else {
            "╠"
        };
        let header_sep_right = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "┤"
        } else {
            "╣"
        };

        let horizontal_line = horizontal.repeat(content_width);
        let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
        let header_sep = format!(
            "{}{}{}\n",
            header_sep_left, horizontal_line, header_sep_right
        );
        let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

        // Calculate visual width of horizontal line
        let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);

        // Center the title
        let title_line = create_content_line(
            left_border,
            title,
            right_border,
            horizontal_visual_width,
            true,
        );

        // Split content into lines and pad each line with 1 space margin on each side
        let content_lines = content
            .lines()
            .map(|line| {
                let line_with_margin = format!(" {} ", line);
                create_content_line(
                    left_border,
                    &line_with_margin,
                    right_border,
                    horizontal_visual_width,
                    false,
                )
            })
            .collect::<String>();

        format!(
            "{}{}{}{}{}",
            top, title_line, header_sep, content_lines, bottom
        )
    }

    pub fn create_nav_box(items: &[NavItem], width: usize) -> String {
        let actual_width = width.max(10); // Minimum width of 10 characters
        let content_width = actual_width.saturating_sub(2);

        let (top_left, horizontal, top_right, left_border, right_border) =
            get_box_chars(actual_width);
        let bottom_left = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "└"
        } else {
            "╚"
        };
        let bottom_right = if actual_width < 20 {
            "+"
        } else if actual_width < 40 {
            "┘"
        } else {
            "╝"
        };

        let nav_items = items
            .iter()
            .map(|item| format!("<a href=\"{}\">{}</a>", item.href, item.text))
            .collect::<Vec<_>>();

        let nav_content = nav_items.join(" | ");

        let horizontal_line = horizontal.repeat(content_width);
        let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
        let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

        // Calculate visual width of horizontal line
        let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);

        // Center the navigation items
        let content_line = create_content_line(
            left_border,
            &nav_content,
            right_border,
            horizontal_visual_width,
            true,
        )
        .trim_end()
        .to_string();

        format!("{}{}\n{}", top, content_line, bottom)
    }
}

pub mod stars {
    use rand::{rng, Rng};
    use serde::Serialize;

    const STAR_CHARS: &[&str] = &[
        "✦", "✧", "★", "☆", "✯", "✡", "✵", "❋", "❆", "❉", "✺", "✹", "✸", "✶", "✷", "✵", "✴", "✳",
        "*", ".", "⋆", "+", "˚",
    ];

    #[derive(Serialize)]
    pub struct Star {
        pub x: i32,
        pub y: i32,
        pub char: String,
        pub delay: f32,
        pub size: f32,
    }

    //pub fn generate_stars(count: usize) -> Vec<Star> {
    pub fn generate_stars(count: usize) -> Vec<Star> {
        let mut stars = Vec::with_capacity(count);
        let mut rng = rng();
        for _ in 0..count {
            let star = Star {
                x: rng.random_range(0..100), // Percentage of screen width
                y: rng.random_range(0..100), // Percentage of screen height
                char: STAR_CHARS[rng.random_range(0..STAR_CHARS.len())].to_string(),
                delay: rng.random_range(0.0..5.0), // Random animation delay up to 5 seconds
                size: rng.random_range(0.8..2.0),  // Random size multiplier
            };
            stars.push(star);
        }

        stars
    }
}

#[get("/")]
fn index() -> Template {
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
        // placeholder post data
        // TODO:Replace with real data source calling from markdown
        title: ascii::replace_problematic_chars("Testing Emoji Support 🚀🔥"),
        href: "#".to_string(),
        date: "October 12, 2025".to_string(),
        content: ascii::replace_problematic_chars(
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

    let footer_text = ascii::replace_problematic_chars(
        "🄯 222.121.87.in-addr.arpa | Created with Rust, <3 and lots of button presses",
    );

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
            "Hello and welcome to my website!\n\nThis is a Rust-powered ASCII art website.",
        ),
        latest_post_box: BoxSizes {
            tiny: {
                let wrapped_content = ascii::wrap_text(&latest_post.content, 22).join("\n");
                ascii::create_header_box(
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
                let wrapped_content = ascii::wrap_text(&latest_post.content, 32).join("\n");
                ascii::create_header_box(
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
                let wrapped_content = ascii::wrap_text(&latest_post.content, 52).join("\n");
                ascii::create_header_box(
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
                let wrapped_content = ascii::wrap_text(&latest_post.content, 72).join("\n");
                ascii::create_header_box(
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
            tiny: ascii::create_header_box("ABOUT ME", "I press buttons.", 25),
            small: ascii::create_header_box("ABOUT ME", "I press buttons.", 30),
            medium: ascii::create_header_box("ABOUT ME", "I press buttons.", 35),
            large: ascii::create_header_box("ABOUT ME", "I press buttons.", 35),
        },
        categories_box: BoxSizes {
            tiny: ascii::create_header_box(
                "CATEGORIES",
                &categories
                    .iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                25,
            ),
            small: ascii::create_header_box(
                "CATEGORIES",
                &categories
                    .iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                30,
            ),
            medium: ascii::create_header_box(
                "CATEGORIES",
                &categories
                    .iter()
                    .map(|cat| format!("• {}", cat))
                    .collect::<Vec<_>>()
                    .join("\n"),
                35,
            ),
            large: ascii::create_header_box(
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
            tiny: ascii::create_header_box(
                "LATEST COMMENTS",
                &format!(
                    "@{}:\n\"{}\"",
                    latest_comment.username, latest_comment.content
                ),
                25,
            ),
            small: ascii::create_header_box(
                "LATEST COMMENTS",
                &format!(
                    "@{}:\n\"{}\"",
                    latest_comment.username, latest_comment.content
                ),
                30,
            ),
            medium: ascii::create_header_box(
                "LATEST COMMENTS",
                &format!(
                    "@{}:\n\"{}\"",
                    latest_comment.username, latest_comment.content
                ),
                35,
            ),
            large: ascii::create_header_box(
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

    Template::render("index", context)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/static", FileServer::from(relative!("static")))
        .attach(Template::fairing())
}
