use crate::mods::{PageContext, BoxSizes, Star, Post};
use tera::Context;

pub struct TemplateContextBuilder {
    context: Context,
}

impl TemplateContextBuilder {
    pub fn new() -> Self {
        Self {
            context: Context::new(),
        }
    }

    pub fn with_page_context(mut self, ctx: &PageContext) -> Self {
        self.context.insert("title_art", &ctx.title_art);
        self.context.insert("navigation_box", &ctx.navigation_box);
        self.context.insert("welcome_box", &ctx.welcome_box);
        self.context.insert("latest_post_box", &ctx.latest_post_box);
        self.context.insert("about_box", &ctx.about_box);
        self.context.insert("categories_box", &ctx.categories_box);
        self.context.insert("comments_box", &ctx.comments_box);
        self.context.insert("footer_box", &ctx.footer_box);
        self.context.insert("stars", &ctx.stars);
        self.context.insert("posts", &ctx.posts);
        self.context.insert("additional_posts", &ctx.additional_posts);
        self
    }

    pub fn with_title_art(mut self, art: &str) -> Self {
        self.context.insert("title_art", art);
        self
    }

    pub fn with_navigation(mut self, nav: &BoxSizes) -> Self {
        self.context.insert("navigation_box", nav);
        self
    }

    pub fn with_welcome(mut self, welcome: &BoxSizes) -> Self {
        self.context.insert("welcome_box", welcome);
        self
    }

    pub fn with_posts(mut self, posts: &BoxSizes) -> Self {
        self.context.insert("latest_post_box", posts);
        self
    }

    pub fn with_about(mut self, about: &BoxSizes) -> Self {
        self.context.insert("about_box", about);
        self
    }

    pub fn with_categories(mut self, categories: &BoxSizes) -> Self {
        self.context.insert("categories_box", categories);
        self
    }

    pub fn with_comments(mut self, comments: &BoxSizes) -> Self {
        self.context.insert("comments_box", comments);
        self
    }

    pub fn with_footer(mut self, footer: &BoxSizes) -> Self {
        self.context.insert("footer_box", footer);
        self
    }

    pub fn with_stars(mut self, stars: &[Star]) -> Self {
        self.context.insert("stars", stars);
        self
    }

    pub fn with_posts_data(mut self, posts: &[Post]) -> Self {
        self.context.insert("posts", posts);
        self
    }

    pub fn with_additional_posts(mut self, posts: &[Post]) -> Self {
        self.context.insert("additional_posts", posts);
        self
    }

    pub fn build(self) -> Context {
        self.context
    }
}

impl Default for TemplateContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
