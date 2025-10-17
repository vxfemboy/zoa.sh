use serde::Serialize;

// Re-export all modules
pub mod ascii_art;
pub mod stars;
pub mod wasm;
pub mod constants;
pub mod data;
pub mod responsive;
pub mod content;
pub mod errors;
pub mod config;
pub mod template_builder;
pub mod cache;
pub mod api;

#[cfg(test)]
mod tests;

// Re-export commonly used items
pub use ascii_art::*;
pub use stars::*;
pub use data::*;
pub use responsive::*;
pub use content::*;
pub use errors::*;
pub use config::*;
pub use template_builder::*;
pub use cache::*;
pub use api::*;

#[derive(Serialize)]
pub struct BoxSizes {
    pub tiny: String,   // For very small screens (<300px)
    pub small: String,  // For small screens (300-600px)
    pub medium: String, // For medium screens (600-900px)
    pub large: String,  // For large screens (>900px)
}

#[derive(Serialize)]
pub struct PageContext {
    pub title_art: String,
    pub navigation_box: BoxSizes,
    pub welcome_box: BoxSizes,
    pub latest_post_box: BoxSizes,
    pub about_box: BoxSizes,
    pub categories_box: BoxSizes,
    pub comments_box: BoxSizes,
    pub footer_box: BoxSizes,
    pub stars: Vec<Star>,
    pub posts: Vec<Post>,
    pub additional_posts: Vec<Post>,
}

#[derive(Serialize)]
pub struct NavItem {
    pub text: String,
    pub href: String,
}

#[derive(Serialize, Clone)]
pub struct Post {
    pub title: String,
    pub date: String,
    pub content: String,
    pub href: String,
}

#[derive(Serialize)]
pub struct Comment {
    pub username: String,
    pub content: String,
}
