#[cfg(test)]
mod unit_tests {

    use crate::mods::*;

    #[test]
    fn test_wrap_text_basic() {
        let text = "Hello world this is a test";
        let wrapped = wrap_text(text, 10);

        assert!(!wrapped.is_empty());
        assert!(wrapped.iter().all(|line| line.len() <= 10));
    }

    #[test]
    fn test_wrap_text_with_emoji() {
        let text = "Hello 🚀 world 🌍 test";
        let wrapped = wrap_text(text, 15);

        assert!(!wrapped.is_empty());
        // Note: Visual width calculation is complex with emojis
    }

    #[test]
    fn test_create_header_box() {
        let box_content = create_header_box("TEST", "This is test content", 20);

        assert!(box_content.contains("TEST"));
        assert!(box_content.contains("This is test content"));
        assert!(
            box_content.contains("╔") || box_content.contains("┌") || box_content.contains("+")
        );
    }

    #[test]
    fn test_create_footer_box() {
        let box_content = create_footer_box("Footer text", 15);

        assert!(box_content.contains("Footer text"));
        assert!(
            box_content.contains("╚") || box_content.contains("└") || box_content.contains("+")
        );
    }

    #[test]
    fn test_replace_problematic_chars() {
        let text = "Hello 🚀 world";
        let replaced = replace_problematic_chars(text);

        assert!(replaced.contains("Hello"));
        assert!(replaced.contains("world"));
        // Should contain img tag for emoji
        assert!(replaced.contains("<img"));
    }

    #[test]
    fn test_generate_stars() {
        let stars = generate_stars(10);

        assert_eq!(stars.len(), 10);
        assert!(stars.iter().all(|star| star.x >= 0 && star.x <= 100));
        assert!(stars.iter().all(|star| star.y >= 0 && star.y <= 100));
    }

    #[test]
    fn test_responsive_boxes() {
        let boxes = ResponsiveBoxes::new_header("TEST", "Content");

        assert!(!boxes.tiny.is_empty());
        assert!(!boxes.small.is_empty());
        assert!(!boxes.medium.is_empty());
        assert!(!boxes.large.is_empty());
    }

    #[test]
    fn test_content_manager() {
        let manager = ContentManager::new();
        let context = manager
            .create_page_context()
            .expect("Failed to create context");

        assert!(!context.title_art.is_empty());
        assert!(!context.posts.is_empty());
        assert!(!context.stars.is_empty());
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.content.star_count, 150);
    }

    #[test]
    fn test_box_cache() {
        let cache = BoxCache::new();

        let size = cache.size();
        assert!(size.is_ok());
        assert_eq!(size.unwrap(), 0);
    }

    #[test]
    fn test_template_context_builder() {
        let builder = TemplateContextBuilder::new()
            .with_title_art("Test Art")
            .with_stars(&generate_stars(5));

        let context = builder.build();

        assert!(context.get("title_art").is_some());
        assert!(context.get("stars").is_some());
    }
}
