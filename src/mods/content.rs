use std::fs;
use crate::mods::{
    SiteData, ResponsiveBoxes, PageContext, Post, wrap_text, replace_problematic_chars, generate_stars,
};
use crate::mods::constants::*;

pub struct ContentManager {
    data: SiteData,
}

impl ContentManager {
    pub fn new() -> Self {
        Self {
            data: SiteData::new(),
        }
    }

    pub fn create_page_context(&self) -> Result<PageContext, Box<dyn std::error::Error>> {
        // Load the ASCII title art from file
        let title_art = fs::read_to_string("templates/ascii/title.txt")
            .unwrap_or_else(|_| "ASCII Web".to_string());

        // Process posts with emoji replacement
        let processed_posts: Vec<Post> = self.data.posts.iter().map(|post| Post {
            title: replace_problematic_chars(&post.title),
            href: post.href.clone(),
            date: post.date.clone(),
            content: replace_problematic_chars(&post.content),
        }).collect();

        // Create responsive boxes
        let navigation_box = ResponsiveBoxes::new_navigation(&self.data.nav_items);
        let welcome_box = ResponsiveBoxes::new_header("WELCOME", &self.data.welcome_content);
        let about_box = ResponsiveBoxes::new_about(&self.data.about_content);
        let categories_box = ResponsiveBoxes::new_categories(&self.data.categories);
        
        let latest_comment = &self.data.comments[0];
        let comments_box = ResponsiveBoxes::new_comments(&latest_comment.username, &latest_comment.content);
        
        let footer_text = replace_problematic_chars(&self.data.footer_text);
        let footer_box = ResponsiveBoxes::new_footer(&footer_text);

        // Create posts content for different screen sizes
        let latest_post_box = self.create_posts_content(&processed_posts);

        // Generate stars
        let stars = generate_stars(DEFAULT_STAR_COUNT);

        // Create additional posts
        let additional_posts: Vec<Post> = processed_posts.iter().skip(1).take(3).cloned().collect();

        Ok(PageContext {
            title_art,
            navigation_box: navigation_box.into(),
            welcome_box: welcome_box.into(),
            latest_post_box: latest_post_box.into(),
            about_box: about_box.into(),
            categories_box: categories_box.into(),
            comments_box: comments_box.into(),
            footer_box: footer_box.into(),
            stars,
            posts: processed_posts,
            additional_posts,
        })
    }

    fn create_posts_content(&self, posts: &[Post]) -> ResponsiveBoxes {
        // Create dividers
        let divider_tiny = "░".repeat(DIVIDER_LENGTH_TINY);
        let divider_small = "░".repeat(DIVIDER_LENGTH_SMALL);
        let divider_medium = "░".repeat(DIVIDER_LENGTH_MEDIUM);
        let divider_large = "░".repeat(DIVIDER_LENGTH_LARGE);

        // Mobile content - only latest post initially
        let latest_post = &posts[0];
        
        let mobile_content_tiny = format!(
            "\n{}\n\n{}\n{}\n\n{}\n\nRead more: {}\n\n{}\n\n┌────────────────────────┐\n│ ▼ SHOW MORE POSTS ▼   │\n└────────────────────────┘",
            divider_tiny,
            latest_post.title,
            latest_post.date,
            wrap_text(&latest_post.content, WRAP_WIDTH_TINY).join("\n"),
            format!("<a href=\"{}\">{}</a>", latest_post.href, latest_post.title),
            divider_tiny
        );

        let mobile_content_small = format!(
            "\n{}\n\n{}\n{}\n\n{}\n\nRead more: {}\n\n{}\n\n┌────────────────────────────────┐\n│ ▼ SHOW MORE POSTS ▼           │\n└────────────────────────────────┘",
            divider_small,
            latest_post.title,
            latest_post.date,
            wrap_text(&latest_post.content, WRAP_WIDTH_SMALL).join("\n"),
            format!("<a href=\"{}\">{}</a>", latest_post.href, latest_post.title),
            divider_small
        );

        // Desktop content - all posts
        let all_posts_content_medium = format!(
            "\n{}\n\n{}\n\n{}",
            divider_medium,
            posts.iter().map(|post| {
                format!(
                    "{}\n{}\n\n{}\n\nRead more: {}",
                    post.title,
                    post.date,
                    wrap_text(&post.content, WRAP_WIDTH_MEDIUM).join("\n"),
                    format!("<a href=\"{}\">{}</a>", post.href, post.title)
                )
            }).collect::<Vec<_>>().join(&format!("\n\n{}\n\n", divider_medium)),
            divider_medium
        );

        let all_posts_content_large = format!(
            "\n{}\n\n{}\n\n{}",
            divider_large,
            posts.iter().map(|post| {
                format!(
                    "{}\n{}\n\n{}\n\nRead more: {}",
                    post.title,
                    post.date,
                    wrap_text(&post.content, WRAP_WIDTH_LARGE).join("\n"),
                    format!("<a href=\"{}\">{}</a>", post.href, post.title)
                )
            }).collect::<Vec<_>>().join(&format!("\n\n{}\n\n", divider_large)),
            divider_large
        );

        ResponsiveBoxes {
            tiny: crate::mods::create_header_box("LATEST POSTS", &mobile_content_tiny, BOX_WIDTH_TINY),
            small: crate::mods::create_header_box("LATEST POSTS", &mobile_content_small, BOX_WIDTH_SMALL),
            medium: crate::mods::create_header_box("LATEST POSTS", &all_posts_content_medium, BOX_WIDTH_MEDIUM),
            large: crate::mods::create_header_box("LATEST POSTS", &all_posts_content_large, BOX_WIDTH_LARGE),
        }
    }
}

impl From<ResponsiveBoxes> for crate::mods::BoxSizes {
    fn from(responsive: ResponsiveBoxes) -> Self {
        crate::mods::BoxSizes {
            tiny: responsive.tiny,
            small: responsive.small,
            medium: responsive.medium,
            large: responsive.large,
        }
    }
}
