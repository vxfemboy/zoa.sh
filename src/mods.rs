use serde::Serialize;

// Re-export all modules
pub mod about;
pub mod api;
pub mod ascii_art;
pub mod cache;
pub mod config;
pub mod constants;
pub mod content;
pub mod data;
pub mod errors;
pub mod image_converter;
pub mod markdown;
pub mod now;
pub mod projects;
pub mod responsive;
pub mod stars;
pub mod template_builder;
pub mod text;
pub mod uses;
pub mod wasm;

#[cfg(test)]
mod tests;

// Re-export commonly used items
pub use api::*;
pub use ascii_art::*;
pub use cache::*;
pub use config::*;
pub use content::*;
pub use data::{Post, SiteData};
pub use errors::*;
pub use responsive::*;
pub use stars::*;
pub use template_builder::*;

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
    pub footer_box: BoxSizes,
    pub stars: Vec<Star>,
    pub posts: Vec<Post>,
    pub additional_posts: Vec<Post>,
}
