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
            .create_page_context("https://vx.gay")
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
    fn test_site_resolve_local_dev_stays_on_host() {
        // localhost keeps the exact host+port over http (no bounce to vx.gay).
        let d = site::resolve("localhost:8084");
        assert_eq!(d.base_url, "http://localhost:8084");
        assert_eq!(d.domain, "localhost:8084");
        assert_eq!(
            site::resolve("127.0.0.1:8080").base_url,
            "http://127.0.0.1:8080"
        );
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
    fn test_about_boxes_email_swaps_by_site() {
        let v = site::resolve("vx.gay");
        let boxes = about::build_about_boxes(&v);
        assert!(boxes.links_box.large.contains("z@vx.gay"));
        assert!(!boxes.links_box.large.contains("zoa@zoa.sh"));
        // repo link literal untouched
        assert!(boxes.links_box.large.contains("github.com/vxfemboy"));
    }

    #[test]
    fn test_markdown_rewrites_relative_asset_images() {
        // markdown_to_html is private; test via the public loader against a temp file.
        let dir = std::env::temp_dir().join("zoa-md-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.md");
        // Cover quoted and bare/unquoted src forms (posts use both).
        std::fs::write(
            &p,
            "---\ntitle: X\nslug: x\nsocial: foo/social.png\n---\n\n<img src=\"assets/foo/1.png\">\n<img src=assets/foo/bare.png>",
        )
        .unwrap();
        let post = markdown::load_markdown_file(&p).unwrap();
        assert_eq!(post.social.as_deref(), Some("foo/social.png"));
        assert!(post.content_html.contains("src=\"/post/assets/foo/1.png\""));
        assert!(!post.content_html.contains("src=\"assets/foo/1.png\""));
        // bare/unquoted src is rewritten too
        assert!(post.content_html.contains("src=/post/assets/foo/bare.png"));
        assert!(!post.content_html.contains("src=assets/foo/bare.png"));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn test_slugify() {
        assert_eq!(
            markdown::slugify("registry vs registrar, and what EPP is"),
            "registry-vs-registrar-and-what-epp-is"
        );
        assert_eq!(markdown::slugify("Filmtek Cloud"), "filmtek-cloud");
        assert_eq!(markdown::slugify("  --Hello,  World!!  "), "hello-world");
        assert_eq!(markdown::slugify("C++ & Rust"), "c-rust");
        assert_eq!(markdown::slugify(""), "");
    }

    #[test]
    fn test_markdown_heading_anchors() {
        let dir = std::env::temp_dir().join("zoa-md-head");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("h.md");
        std::fs::write(
            &p,
            "---\ntitle: T\nslug: t\n---\n\n# Title\n\n## Foo Bar\n\ntext\n\n## Foo Bar\n\n### Baz",
        )
        .unwrap();
        let post = markdown::load_markdown_file(&p).unwrap();
        let h = &post.content_html;
        // h2 gets an id, and renders as a plain heading (no injected anchor / `#`)
        assert!(h.contains("<h2 id=\"foo-bar\">Foo Bar</h2>"));
        // no clickable anchor or stray `#` is injected into headings
        assert!(!h.contains("heading-anchor"));
        assert!(!h.contains("heading-link"));
        assert!(!h.contains(">#</a>"));
        // duplicate heading text is deduped
        assert!(h.contains("<h2 id=\"foo-bar-2\">"));
        // h3 gets an id too
        assert!(h.contains("<h3 id=\"baz\">Baz</h3>"));
        // h1 (post title) is NOT given an id
        assert!(h.contains("<h1"));
        assert!(!h.contains("id=\"title\""));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn test_company_bit_map_is_stable_and_alphabetical() {
        let m = about::company_bit_map();
        // every bit is unique and contiguous 0..N
        let mut bits: Vec<u32> = m.values().copied().collect();
        bits.sort_unstable();
        for (i, b) in bits.iter().enumerate() {
            assert_eq!(*b, i as u32, "bits must be contiguous 0..N");
        }
        // alphabetical: an earlier slug has a lower bit than a later one
        assert!(m["canyons-school-district"] < m["nickelcade"]);
        // rendered jobs carry data-bit
        assert!(about::experience_box(78, false).contains("data-bit="));
    }
}
