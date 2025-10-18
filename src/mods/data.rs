use crate::mods::image_converter::ImageConverter;
use serde::Serialize;

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

pub struct SiteData {
    pub nav_items: Vec<NavItem>,
    pub posts: Vec<Post>,
    pub categories: Vec<String>,
    pub comments: Vec<Comment>,
    pub footer_text: String,
    pub about_content: String,
    pub about_ascii_art: String,
    pub welcome_content: String,
}

impl Default for SiteData {
    fn default() -> Self {
        Self::new()
    }
}

impl SiteData {
    pub fn new() -> Self {
        // Convert profile image to ASCII art
        let image_converter = ImageConverter::new();
        let profile_ascii = image_converter
            .convert_profile_image("static/profile.png")
            .unwrap_or_else(|_| "ASCII profile image not available".to_string());

        Self {
            nav_items: vec![
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
                    href: "https://github.com/vxfemboy".to_string(),
                },
                NavItem {
                    text: "BLOG".to_string(),
                    href: "#".to_string(),
                },
                NavItem {
                    text: "CONTACT".to_string(),
                    href: "#".to_string(),
                },
            ],
            posts: vec![
                Post {
                    title: "RIP KAYOS".to_string(),
                    date: "2024-08-18".to_string(),
                    content: "see you in the packet flow old friend...".to_string(),
                    href: "https://soundcloud.com/queed-inc".to_string(),
                },
                Post {
                    title: "a story about a cat".to_string(),
                    date: "2024-08-15".to_string(),
                    content: "meowmeow meow meow meow meow meow meow meow meow meow.".to_string(),
                    href: "https://poptart.cat".to_string(),
                },
                Post {
                    title: "Welcome to My Website".to_string(),
                    date: "2024-08-15".to_string(),
                    content: "This is my first post on this Rust-powered ASCII art website. I'm excited to share my thoughts and projects here.".to_string(),
                    href: "#".to_string(),
                },

            ],
            categories: vec![
                "Software".to_string(),
                "Network".to_string(),
                "Security".to_string(),
                "Hardware".to_string(),
            ],
            comments: vec![
                Comment {
                    username: "rustacean".to_string(),
                    content: "Love the ASCII aesthetic!".to_string(),
                },
                Comment {
                    username: "terminal_lover".to_string(),
                    content: "This brings back memories of the old terminal days. Beautiful implementation!".to_string(),
                },
            ],
            footer_text: "🄯 vxfemboy | meow <3".to_string(),
            about_content: "I press buttons.".to_string(),
            about_ascii_art: profile_ascii,
            welcome_content: "Hello and welcome to my website!\n\nlol i wanna die.".to_string(),
        }
    }
}
