use crate::mods::constants::*;
use crate::mods::data::NavItem;
use crate::mods::{create_nav_box, BoxBuilder, BoxStyle};

pub struct ResponsiveBoxes {
    pub tiny: String,
    pub small: String,
    pub medium: String,
    pub large: String,
}

impl ResponsiveBoxes {
    pub fn new_header(title: &str, content: &str) -> Self {
        Self {
            tiny: BoxBuilder::new()
                .with_title(title)
                .with_content(content)
                .with_width(BOX_WIDTH_TINY)
                .with_style(BoxStyle::Header)
                .build(),
            small: BoxBuilder::new()
                .with_title(title)
                .with_content(content)
                .with_width(BOX_WIDTH_SMALL)
                .with_style(BoxStyle::Header)
                .build(),
            medium: BoxBuilder::new()
                .with_title(title)
                .with_content(content)
                .with_width(BOX_WIDTH_MEDIUM)
                .with_style(BoxStyle::Header)
                .build(),
            large: BoxBuilder::new()
                .with_title(title)
                .with_content(content)
                .with_width(BOX_WIDTH_LARGE)
                .with_style(BoxStyle::Header)
                .build(),
        }
    }

    pub fn new_footer(content: &str) -> Self {
        Self {
            tiny: BoxBuilder::new()
                .with_content(content)
                .with_width(BOX_WIDTH_TINY)
                .with_style(BoxStyle::Footer)
                .build(),
            small: BoxBuilder::new()
                .with_content(content)
                .with_width(BOX_WIDTH_SMALL)
                .with_style(BoxStyle::Footer)
                .build(),
            medium: BoxBuilder::new()
                .with_content(content)
                .with_width(BOX_WIDTH_MEDIUM)
                .with_style(BoxStyle::Footer)
                .build(),
            large: BoxBuilder::new()
                .with_content(content)
                .with_width(BOX_WIDTH_LARGE)
                .with_style(BoxStyle::Footer)
                .build(),
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
            tiny: BoxBuilder::new()
                .with_title("WHOAMI")
                .with_content(text_content)
                .with_ascii_art(ascii_art)
                .with_width(ABOUT_WIDTH_TINY)
                .with_style(BoxStyle::AboutWithAscii)
                .build(),
            small: BoxBuilder::new()
                .with_title("WHOAMI")
                .with_content(text_content)
                .with_ascii_art(ascii_art)
                .with_width(ABOUT_WIDTH_SMALL)
                .with_style(BoxStyle::AboutWithAscii)
                .build(),
            medium: BoxBuilder::new()
                .with_title("WHOAMI")
                .with_content(text_content)
                .with_ascii_art(ascii_art)
                .with_width(ABOUT_WIDTH_MEDIUM)
                .with_style(BoxStyle::AboutWithAscii)
                .build(),
            large: BoxBuilder::new()
                .with_title("WHOAMI")
                .with_content(text_content)
                .with_ascii_art(ascii_art)
                .with_width(ABOUT_WIDTH_LARGE)
                .with_style(BoxStyle::AboutWithAscii)
                .build(),
        }
    }

    pub fn new_categories(categories: &[String]) -> Self {
        let content = categories
            .iter()
            .map(|cat| format!("• {}", cat))
            .collect::<Vec<_>>()
            .join("\n");

        Self {
            tiny: BoxBuilder::new()
                .with_title("CATEGORIES")
                .with_content(&content)
                .with_width(CATEGORIES_WIDTH_TINY)
                .with_style(BoxStyle::Header)
                .build(),
            small: BoxBuilder::new()
                .with_title("CATEGORIES")
                .with_content(&content)
                .with_width(CATEGORIES_WIDTH_SMALL)
                .with_style(BoxStyle::Header)
                .build(),
            medium: BoxBuilder::new()
                .with_title("CATEGORIES")
                .with_content(&content)
                .with_width(CATEGORIES_WIDTH_MEDIUM)
                .with_style(BoxStyle::Header)
                .build(),
            large: BoxBuilder::new()
                .with_title("CATEGORIES")
                .with_content(&content)
                .with_width(CATEGORIES_WIDTH_LARGE)
                .with_style(BoxStyle::Header)
                .build(),
        }
    }

    #[allow(dead_code)]
    pub fn new_comments(username: &str, content: &str) -> Self {
        let comment_content = format!("@{}:\n\"{}\"", username, content);

        Self {
            tiny: BoxBuilder::new()
                .with_title("LATEST COMMENTS")
                .with_content(&comment_content)
                .with_width(COMMENTS_WIDTH_TINY)
                .with_style(BoxStyle::Header)
                .build(),
            small: BoxBuilder::new()
                .with_title("LATEST COMMENTS")
                .with_content(&comment_content)
                .with_width(COMMENTS_WIDTH_SMALL)
                .with_style(BoxStyle::Header)
                .build(),
            medium: BoxBuilder::new()
                .with_title("LATEST COMMENTS")
                .with_content(&comment_content)
                .with_width(COMMENTS_WIDTH_MEDIUM)
                .with_style(BoxStyle::Header)
                .build(),
            large: BoxBuilder::new()
                .with_title("LATEST COMMENTS")
                .with_content(&comment_content)
                .with_width(COMMENTS_WIDTH_LARGE)
                .with_style(BoxStyle::Header)
                .build(),
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
            tiny: BoxBuilder::new()
                .with_title("SHOUTBOX")
                .with_content(shoutbox_content)
                .with_width(COMMENTS_WIDTH_TINY)
                .with_style(BoxStyle::Header)
                .build(),
            small: BoxBuilder::new()
                .with_title("SHOUTBOX")
                .with_content(shoutbox_content)
                .with_width(COMMENTS_WIDTH_SMALL)
                .with_style(BoxStyle::Header)
                .build(),
            medium: BoxBuilder::new()
                .with_title("SHOUTBOX")
                .with_content(shoutbox_content)
                .with_width(COMMENTS_WIDTH_MEDIUM)
                .with_style(BoxStyle::Header)
                .build(),
            large: BoxBuilder::new()
                .with_title("SHOUTBOX")
                .with_content(shoutbox_content)
                .with_width(COMMENTS_WIDTH_LARGE)
                .with_style(BoxStyle::Header)
                .build(),
        }
    }
}
