use crate::mods::constants::*;
use crate::mods::data::NavItem;
use crate::mods::{
    create_about_box_with_ascii, create_footer_box, create_header_box, create_nav_box,
};

pub struct ResponsiveBoxes {
    pub tiny: String,
    pub small: String,
    pub medium: String,
    pub large: String,
}

impl ResponsiveBoxes {
    pub fn new_header(title: &str, content: &str) -> Self {
        Self {
            tiny: create_header_box(title, content, BOX_WIDTH_TINY),
            small: create_header_box(title, content, BOX_WIDTH_SMALL),
            medium: create_header_box(title, content, BOX_WIDTH_MEDIUM),
            large: create_header_box(title, content, BOX_WIDTH_LARGE),
        }
    }

    pub fn new_footer(content: &str) -> Self {
        Self {
            tiny: create_footer_box(content, BOX_WIDTH_TINY),
            small: create_footer_box(content, BOX_WIDTH_SMALL),
            medium: create_footer_box(content, BOX_WIDTH_MEDIUM),
            large: create_footer_box(content, BOX_WIDTH_LARGE),
        }
    }

    pub fn new_navigation(items: &[NavItem]) -> Self {
        Self {
            tiny: create_nav_box(items, NAV_WIDTH_TINY),
            small: create_nav_box(items, NAV_WIDTH_SMALL),
            medium: create_nav_box(items, NAV_WIDTH_MEDIUM),
            large: create_nav_box(items, NAV_WIDTH_LARGE),
        }
    }

    pub fn new_about_with_ascii(text_content: &str, ascii_art: &str) -> Self {
        Self {
            tiny: create_about_box_with_ascii(
                "WHOAMI",
                text_content,
                ascii_art,
                ABOUT_WIDTH_TINY,
            ),
            small: create_about_box_with_ascii(
                "WHOAMI",
                text_content,
                ascii_art,
                ABOUT_WIDTH_SMALL,
            ),
            medium: create_about_box_with_ascii(
                "WHOAMI",
                text_content,
                ascii_art,
                ABOUT_WIDTH_MEDIUM,
            ),
            large: create_about_box_with_ascii(
                "WHOAMI",
                text_content,
                ascii_art,
                ABOUT_WIDTH_LARGE,
            ),
        }
    }

    pub fn new_categories(categories: &[String]) -> Self {
        let content = categories
            .iter()
            .map(|cat| format!("• {}", cat))
            .collect::<Vec<_>>()
            .join("\n");

        Self {
            tiny: create_header_box("CATEGORIES", &content, CATEGORIES_WIDTH_TINY),
            small: create_header_box("CATEGORIES", &content, CATEGORIES_WIDTH_SMALL),
            medium: create_header_box("CATEGORIES", &content, CATEGORIES_WIDTH_MEDIUM),
            large: create_header_box("CATEGORIES", &content, CATEGORIES_WIDTH_LARGE),
        }
    }

    pub fn new_comments(username: &str, content: &str) -> Self {
        let comment_content = format!("@{}:\n\"{}\"", username, content);

        Self {
            tiny: create_header_box("LATEST COMMENTS", &comment_content, COMMENTS_WIDTH_TINY),
            small: create_header_box("LATEST COMMENTS", &comment_content, COMMENTS_WIDTH_SMALL),
            medium: create_header_box("LATEST COMMENTS", &comment_content, COMMENTS_WIDTH_MEDIUM),
            large: create_header_box("LATEST COMMENTS", &comment_content, COMMENTS_WIDTH_LARGE),
        }
    }

    pub fn new_shoutbox() -> Self {
        let shoutbox_content = "╔══════════════════════════════════════╗
║           SHOUTBOX                   ║
╠══════════════════════════════════════╣
║                                      ║
║  Connect to join the conversation!   ║
║                                      ║
║  Type your message below:            ║
║  [username] [message]                ║
║                                      ║
║  Example:                            ║
║  rustacean Hello world!              ║
║                                      ║
║  Messages will appear here in        ║
║  real-time as they're posted...      ║
║                                      ║
╚══════════════════════════════════════╝";

        Self {
            tiny: create_header_box("SHOUTBOX", shoutbox_content, COMMENTS_WIDTH_TINY),
            small: create_header_box("SHOUTBOX", shoutbox_content, COMMENTS_WIDTH_SMALL),
            medium: create_header_box("SHOUTBOX", shoutbox_content, COMMENTS_WIDTH_MEDIUM),
            large: create_header_box("SHOUTBOX", shoutbox_content, COMMENTS_WIDTH_LARGE),
        }
    }
}
