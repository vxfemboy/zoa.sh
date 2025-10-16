use serde::Serialize;

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
    pub stars: Vec<stars::Star>,
}

#[derive(Serialize)]
pub struct NavItem {
    pub text: String,
    pub href: String,
}

#[derive(Serialize)]
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
