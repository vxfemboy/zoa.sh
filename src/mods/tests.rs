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

    #[test]
    fn test_site_resolve_known_and_default() {
        let z = site::resolve("zoa.sh");
        assert_eq!(z.domain, "zoa.sh");
        assert_eq!(z.base_url, "https://zoa.sh");
        assert_eq!(z.email, "zoa@zoa.sh");

        let v = site::resolve("vx.gay");
        assert_eq!(v.domain, "vx.gay");
        assert_eq!(v.email, "z@vx.gay");

        // namecheap.wtf defers to vx.gay
        let n = site::resolve("namecheap.wtf");
        assert_eq!(n.domain, "vx.gay");
        assert_eq!(n.base_url, "https://vx.gay");

        // unknown / direct-IP → default vx.gay
        assert_eq!(site::resolve("10.75.87.110").domain, "vx.gay");
    }

    #[test]
    fn test_site_resolve_normalizes_host() {
        assert_eq!(site::resolve("ZOA.SH:8084").domain, "zoa.sh");
        assert_eq!(site::resolve("www.vx.gay").domain, "vx.gay");
    }

    #[test]
    fn test_site_apply_tokens_opt_in() {
        let v = site::resolve("vx.gay");
        assert_eq!(v.apply("curl {domain}"), "curl vx.gay");
        assert_eq!(v.apply("mail: {email}"), "mail: z@vx.gay");
        // literals never swap
        assert_eq!(
            v.apply("github.com/vxfemboy/zoa.sh"),
            "github.com/vxfemboy/zoa.sh"
        );
    }

    #[test]
    fn test_site_og_image() {
        let z = site::resolve("zoa.sh");
        assert_eq!(z.og_image(None), "https://zoa.sh/static/og-image.gif");
        assert_eq!(
            z.og_image(Some("namecheap/social.png")),
            "https://zoa.sh/post/assets/namecheap/social.png"
        );
    }

    #[test]
    fn test_markdown_rewrites_relative_asset_images() {
        // markdown_to_html is private; test via the public loader against a temp file.
        let dir = std::env::temp_dir().join("zoa-md-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.md");
        std::fs::write(
            &p,
            "---\ntitle: X\nslug: x\nsocial: foo/social.png\n---\n\n<img src=\"assets/foo/1.png\">",
        )
        .unwrap();
        let post = markdown::load_markdown_file(&p).unwrap();
        assert_eq!(post.social.as_deref(), Some("foo/social.png"));
        assert!(post.content_html.contains("src=\"/post/assets/foo/1.png\""));
        assert!(!post.content_html.contains("src=\"assets/foo/1.png\""));
        std::fs::remove_file(&p).ok();
    }
}
