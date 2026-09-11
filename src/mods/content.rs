use crate::mods::constants::*;
use crate::mods::data::Post;
use crate::mods::{
    generate_stars, replace_problematic_chars, wrap_text, PageContext, ResponsiveBoxes, SiteData,
};
use std::fs;

/// Truncate text to max length at word boundary
fn truncate_content(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let truncated = &text[..max_len];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &truncated[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

pub struct ContentManager {
    data: SiteData,
}

impl Default for ContentManager {
    fn default() -> Self {
        Self::new()
    }
}

fn wrap_title_block(post: &Post, width: usize) -> String {
    let title_block = if let Some(ref sub) = post.subtitle {
        format!("{}\n» {}", post.display_title(), sub)
    } else {
        post.display_title().to_string()
    };
    title_block
        .lines()
        .flat_map(|line| wrap_text(line, width))
        .collect::<Vec<_>>()
        .join("\n")
}

impl ContentManager {
    pub fn new() -> Self {
        Self {
            data: SiteData::new(),
        }
    }

    pub fn create_page_context(
        &self,
        base_url: &str,
    ) -> Result<PageContext, Box<dyn std::error::Error>> {
        // Load the ASCII title art from file
        let title_art = fs::read_to_string("templates/ascii/title.txt")
            .unwrap_or_else(|_| "vxfemboy".to_string());

        // Process posts with emoji replacement
        let processed_posts: Vec<Post> = self
            .data
            .posts
            .iter()
            .map(|post| Post {
                title: replace_problematic_chars(&post.title),
                short_title: post.short_title.as_deref().map(replace_problematic_chars),
                subtitle: post.subtitle.as_deref().map(replace_problematic_chars),
                href: post.href.clone(),
                date: post.date.clone(),
                slug: post.slug.clone(),
                tags: post.tags.clone(),
                content: replace_problematic_chars(&post.content),
                content_html: post.content_html.clone(),
                series: post.series.clone(),
                series_order: post.series_order,
            })
            .collect();

        // Create navigation box (using full URL)
        let navigation_box = ResponsiveBoxes::new_navigation(&self.data.nav_items, base_url);

        // Create welcome and about boxes
        let welcome_box = ResponsiveBoxes::new_header("WELCOME", &self.data.welcome_content);
        let about_box = ResponsiveBoxes::new_about_with_ascii(
            &self.data.about_content,
            &self.data.about_ascii_art,
        );
        let categories_box = ResponsiveBoxes::new_categories(&self.data.categories);

        let footer_text = replace_problematic_chars(&self.data.footer_text);
        let footer_box = ResponsiveBoxes::new_footer(&footer_text);

        // Create posts content for different screen sizes
        let latest_post_box = self.create_posts_content(&processed_posts, base_url);

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
            footer_box: footer_box.into(),
            stars,
            posts: processed_posts,
            additional_posts,
        })
    }

    pub fn create_posts_content(&self, posts: &[Post], base_url: &str) -> ResponsiveBoxes {
        if posts.is_empty() {
            return ResponsiveBoxes {
                tiny: crate::mods::create_header_box("POSTS", "\nNo posts yet.\n", BOX_WIDTH_SMALL),
                small: crate::mods::create_header_box("POSTS", "\nNo posts yet.\n", BOX_WIDTH_MEDIUM),
                medium: crate::mods::create_header_box("POSTS", "\nNo posts yet.\n", BOX_WIDTH_MEDIUM),
                large: crate::mods::create_header_box("POSTS", "\nNo posts yet.\n", BOX_WIDTH_LARGE),
            };
        }

        // Create dividers (match box content widths)
        let divider_tiny = "░".repeat(BOX_WIDTH_SMALL.saturating_sub(4));
        let divider_small = "░".repeat(BOX_WIDTH_MEDIUM.saturating_sub(4));
        let divider_medium = "░".repeat(DIVIDER_LENGTH_MEDIUM);
        let divider_large = "░".repeat(DIVIDER_LENGTH_LARGE);

        // Mobile content - only latest post initially
        let latest_post = &posts[0];

        // Truncate content to 280 chars max before wrapping
        let truncated_content = truncate_content(&latest_post.content, 280);
        let latest_post_href = format!("{}{}", base_url, latest_post.href);

        let title_tiny = wrap_title_block(latest_post, BOX_WIDTH_SMALL.saturating_sub(6));
        let mobile_content_tiny = format!(
            "\n{}\n\n{}\n{}\n\n{}\n\nRead more: <a href=\"{}\">[ View Post >> ]</a>\n\n{}\n\n┌──────────────────────────┐\n│ ▼ SHOW MORE POSTS ▼    │\n└──────────────────────────┘",
            divider_tiny,
            title_tiny,
            latest_post.date,
            wrap_text(&truncated_content, BOX_WIDTH_SMALL.saturating_sub(6)).join("\n"),
            latest_post_href,
            divider_tiny
        );

        let title_small = wrap_title_block(latest_post, BOX_WIDTH_MEDIUM.saturating_sub(6));
        let mobile_content_small = format!(
            "\n{}\n\n{}\n{}\n\n{}\n\nRead more: <a href=\"{}\">[ View Post >> ]</a>\n\n{}\n\n┌──────────────────────────────────────┐\n│  ▼ SHOW MORE POSTS ▼                │\n└──────────────────────────────────────┘",
            divider_small,
            title_small,
            latest_post.date,
            wrap_text(&truncated_content, BOX_WIDTH_MEDIUM.saturating_sub(6)).join("\n"),
            latest_post_href,
            divider_small
        );

        // Desktop content - show only first 3 posts, link to /blog for more
        let display_posts: Vec<_> = posts.iter().take(3).collect();
        let has_more = posts.len() > 3;

        let all_posts_content_medium = format!(
            "\n{}\n\n{}\n\n{}{}",
            divider_medium,
            display_posts
                .iter()
                .map(|post| {
                    let title_block = wrap_title_block(post, WRAP_WIDTH_MEDIUM);
                    let truncated = truncate_content(&post.content, 280);
                    let href = format!("{}{}", base_url, post.href);
                    format!(
                        "{}\n{}\n\n{}\n\nRead more: <a href=\"{}\">[ View Post >> ]</a>",
                        title_block,
                        post.date,
                        wrap_text(&truncated, WRAP_WIDTH_MEDIUM).join("\n"),
                        href,
                    )
                })
                .collect::<Vec<_>>()
                .join(&format!("\n\n{}\n\n", divider_medium)),
            divider_medium,
            if has_more {
                format!(
                    "\n\n<a href=\"/blog\">[ View all {} posts >> ]</a>",
                    posts.len()
                )
            } else {
                String::new()
            }
        );

        let all_posts_content_large = format!(
            "\n{}\n\n{}\n\n{}{}",
            divider_large,
            display_posts
                .iter()
                .map(|post| {
                    let title_block = wrap_title_block(post, WRAP_WIDTH_LARGE);
                    let truncated = truncate_content(&post.content, 280);
                    let href = format!("{}{}", base_url, post.href);
                    format!(
                        "{}\n{}\n\n{}\n\nRead more: <a href=\"{}\">[ View Post >> ]</a>",
                        title_block,
                        post.date,
                        wrap_text(&truncated, WRAP_WIDTH_LARGE).join("\n"),
                        href,
                    )
                })
                .collect::<Vec<_>>()
                .join(&format!("\n\n{}\n\n", divider_large)),
            divider_large,
            if has_more {
                format!(
                    "\n\n<a href=\"/blog\">[ View all {} posts >> ]</a>",
                    posts.len()
                )
            } else {
                String::new()
            }
        );

        ResponsiveBoxes {
            tiny: crate::mods::create_header_box("POSTS", &mobile_content_tiny, BOX_WIDTH_SMALL),
            small: crate::mods::create_header_box("POSTS", &mobile_content_small, BOX_WIDTH_MEDIUM),
            medium: crate::mods::create_header_box(
                "POSTS",
                &all_posts_content_medium,
                BOX_WIDTH_MEDIUM,
            ),
            large: crate::mods::create_header_box(
                "POSTS",
                &all_posts_content_large,
                BOX_WIDTH_LARGE,
            ),
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
