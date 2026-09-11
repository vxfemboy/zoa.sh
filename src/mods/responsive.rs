use crate::mods::constants::*;
use crate::mods::data::NavItem;
use crate::mods::{create_nav_box, create_post_header_box, BoxBuilder, BoxStyle};

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

    pub fn new_navigation(items: &[NavItem], base_url: &str) -> Self {
        Self {
            tiny: create_nav_box(items, NAV_WIDTH_TINY, base_url),
            small: create_nav_box(items, NAV_WIDTH_SMALL, base_url),
            medium: create_nav_box(items, NAV_WIDTH_MEDIUM, base_url),
            large: create_nav_box(items, NAV_WIDTH_LARGE, base_url),
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

    /// Create a post header box with title, date, divider, and back link
    pub fn new_post_header(title: &str, date: &str) -> Self {
        Self {
            tiny: create_post_header_box(title, date, BOX_WIDTH_TINY),
            small: create_post_header_box(title, date, BOX_WIDTH_SMALL),
            medium: create_post_header_box(title, date, BOX_WIDTH_MEDIUM),
            large: create_post_header_box(title, date, BOX_WIDTH_LARGE),
        }
    }

    /// Create an RSS subscription box
    pub fn new_rss_box() -> Self {
        let content = "<a href=\"/rss.xml\">Subscribe via RSS</a>";
        Self {
            tiny: BoxBuilder::new()
                .with_title("RSS")
                .with_content(content)
                .with_width(CATEGORIES_WIDTH_TINY)
                .with_style(BoxStyle::Header)
                .build(),
            small: BoxBuilder::new()
                .with_title("RSS")
                .with_content(content)
                .with_width(CATEGORIES_WIDTH_SMALL)
                .with_style(BoxStyle::Header)
                .build(),
            medium: BoxBuilder::new()
                .with_title("RSS")
                .with_content(content)
                .with_width(CATEGORIES_WIDTH_MEDIUM)
                .with_style(BoxStyle::Header)
                .build(),
            large: BoxBuilder::new()
                .with_title("RSS")
                .with_content(content)
                .with_width(CATEGORIES_WIDTH_LARGE)
                .with_style(BoxStyle::Header)
                .build(),
        }
    }
}
