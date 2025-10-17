use crate::mods::{NavItem, Post, Comment};

pub struct SiteData {
    pub nav_items: Vec<NavItem>,
    pub posts: Vec<Post>,
    pub categories: Vec<String>,
    pub comments: Vec<Comment>,
    pub footer_text: String,
    pub about_content: String,
    pub welcome_content: String,
}

impl SiteData {
    pub fn new() -> Self {
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
            ],
            posts: vec![
                Post {
                    title: "Testing Emoji Support 🚀🔥".to_string(),
                    href: "#".to_string(),
                    date: "October 12, 2025".to_string(),
                    content: "Testing various emojis: 🌈 rainbow, ⭐ star, 💻 laptop, 🎉 party, ❤️ heart, and 🄯 copyleft!".to_string(),
                },
                Post {
                    title: "Building ASCII Art Websites 🎨".to_string(),
                    href: "#".to_string(),
                    date: "October 10, 2025".to_string(),
                    content: "Exploring the retro aesthetic of ASCII art in modern web development. From terminal interfaces to nostalgic design patterns.".to_string(),
                },
                Post {
                    title: "Rust Web Development 🦀".to_string(),
                    href: "#".to_string(),
                    date: "October 8, 2025".to_string(),
                    content: "Why Rust is becoming a popular choice for web backends. Performance, safety, and the joy of systems programming.".to_string(),
                },
                Post {
                    title: "The Art of Minimal Design ✨".to_string(),
                    href: "#".to_string(),
                    date: "October 5, 2025".to_string(),
                    content: "Less is more. How minimal design principles can create powerful user experiences and clean codebases.".to_string(),
                },
            ],
            categories: vec![
                "Kernel".to_string(),
                "Security".to_string(),
                "Networking".to_string(),
                "Systems".to_string(),
                "Research".to_string(),
            ],
            comments: vec![
                Comment {
                    username: "user123".to_string(),
                    content: "Love the ASCII aesthetic!".to_string(),
                },
            ],
            footer_text: "🄯 vxfemboy | meow <3".to_string(),
            about_content: "I press buttons.".to_string(),
            welcome_content: "Hello and welcome to my website!\n\nThis is a Rust-powered ASCII art website.".to_string(),
        }
    }
}
