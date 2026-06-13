use crate::mods::image_converter::ImageConverter;
use crate::mods::markdown;
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
    pub slug: String,
    pub tags: Vec<String>,
    pub content: String,
    pub content_html: String,
    pub href: String,
}

pub struct SiteData {
    pub nav_items: Vec<NavItem>,
    pub posts: Vec<Post>,
    pub categories: Vec<String>,
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

        // Load posts from markdown files
        let md_posts = markdown::load_all_posts("posts");
        let posts: Vec<Post> = md_posts
            .into_iter()
            .map(|mp| Post {
                title: mp.title,
                date: mp.date,
                slug: mp.slug.clone(),
                tags: mp.tags,
                content: mp.content_plain,
                content_html: mp.content_html,
                href: format!("/post/{}", mp.slug),
            })
            .collect();

        Self {
            nav_items: vec![
                NavItem {
                    text: "HOME".to_string(),
                    href: "/".to_string(),
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
                    href: "/blog".to_string(),
                },
                NavItem {
                    text: "CONTACT".to_string(),
                    href: "#".to_string(),
                },
            ],
            posts,
            categories: vec![
                "Software".to_string(),
                "Network".to_string(),
                "Security".to_string(),
                "Hardware".to_string(),
            ],
            footer_text: "🄯 vxfemboy | meow <3".to_string(),
            about_content: "I press buttons.".to_string(),
            about_ascii_art: profile_ascii,
            welcome_content: "Haiiiiiii welcome to my site lol".to_string(),
        }
    }
}
