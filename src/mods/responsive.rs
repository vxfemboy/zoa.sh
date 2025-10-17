use crate::mods::{create_header_box, create_footer_box, create_nav_box, NavItem};
use crate::mods::constants::*;

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

    pub fn new_about(content: &str) -> Self {
        Self {
            tiny: create_header_box("ABOUT ME", content, ABOUT_WIDTH_TINY),
            small: create_header_box("ABOUT ME", content, ABOUT_WIDTH_SMALL),
            medium: create_header_box("ABOUT ME", content, ABOUT_WIDTH_MEDIUM),
            large: create_header_box("ABOUT ME", content, ABOUT_WIDTH_LARGE),
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
}
