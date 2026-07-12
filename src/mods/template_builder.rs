use crate::mods::{PageContext, Star};
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
        self.context.insert("footer_box", &ctx.footer_box);
        self.context.insert("stars", &ctx.stars);
        self.context.insert("posts", &ctx.posts);
        self.context
            .insert("additional_posts", &ctx.additional_posts);
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.context.insert("debug", &debug);
        self
    }

    pub fn with_site(mut self, base_url: &str, canonical: &str, og_image: &str) -> Self {
        self.context.insert("base_url", base_url);
        self.context.insert("canonical", canonical);
        self.context.insert("og_image", og_image);
        self
    }

    #[allow(dead_code)]
    pub fn with_title_art(mut self, art: &str) -> Self {
        self.context.insert("title_art", art);
        self
    }

    #[allow(dead_code)]
    pub fn with_stars(mut self, stars: &[Star]) -> Self {
        self.context.insert("stars", stars);
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
