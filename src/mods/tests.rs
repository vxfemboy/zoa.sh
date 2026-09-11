#[cfg(test)]
mod unit_tests {

    use crate::mods::constants::*;
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
    fn test_header_box_long_title_wrapping() {
        let long_title = "This is a very long post title that exceeds the maximum width of the box by far!";
        assert!(long_title.chars().count() >= 80);
        let box_content = create_header_box(long_title, "Some content", 40);

        let lines: Vec<&str> = box_content.lines().collect();
        assert!(!lines.is_empty());

        for line in &lines {
            assert_eq!(
                count_visual_width_excluding_html(line),
                40,
                "Line visual length was {} instead of 40: '{}'",
                count_visual_width_excluding_html(line),
                line
            );
        }
    }

    #[test]
    fn test_post_header_box_long_title_wrapping() {
        let long_title = "This is a very long post title that exceeds the maximum width of the box by far!";
        let box_content = create_post_header_box(long_title, "2026-09-11", 40);

        let lines: Vec<&str> = box_content.lines().collect();
        assert!(!lines.is_empty());

        for line in &lines {
            assert_eq!(
                count_visual_width_excluding_html(line),
                40,
                "Line visual length was {} instead of 40: '{}'",
                count_visual_width_excluding_html(line),
                line
            );
        }
    }

    #[test]
    fn test_about_box_long_title_wrapping() {
        let long_title = "This is a very long post title that exceeds the maximum width of the box by far!";
        let ascii_art = "                                      ";
        let box_content = create_about_box_with_ascii(long_title, "About content", ascii_art, 40);

        let lines: Vec<&str> = box_content.lines().collect();
        assert!(!lines.is_empty());

        for line in &lines {
            assert_eq!(
                count_visual_width_excluding_html(line),
                40,
                "Line visual length was {} instead of 40: '{}'",
                count_visual_width_excluding_html(line),
                line
            );
        }
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

    #[test]
    fn test_create_posts_content_display_title_subtitle_and_standard_link() {
        let post_with_sub = Post {
            title: "Autonomous System: Part 1 - WAN Clustering Fallacy".to_string(),
            short_title: Some("Autonomous System: Part 1".to_string()),
            subtitle: Some("WAN Clustering Fallacy".to_string()),
            date: "2026-08-14".to_string(),
            slug: "2026-08-14-autonomous-system-part-1-wan-clustering-fallacy".to_string(),
            tags: vec!["network".to_string()],
            content: "First sentence of the post narrative.".to_string(),
            content_html: "<p>First sentence</p>".to_string(),
            href: "/post/2026-08-14-autonomous-system-part-1-wan-clustering-fallacy".to_string(),
            series: Some("Autonomous System".to_string()),
            series_order: Some(1),
        };
        let post_simple = Post {
            title: "Simple Post Without Subtitle".to_string(),
            short_title: None,
            subtitle: None,
            date: "2026-08-15".to_string(),
            slug: "simple-post".to_string(),
            tags: vec![],
            content: "Content of simple post.".to_string(),
            content_html: "<p>Content</p>".to_string(),
            href: "/post/simple-post".to_string(),
            series: None,
            series_order: None,
        };

        let posts = vec![post_with_sub, post_simple];
        let manager = ContentManager::new();
        let boxes = manager.create_posts_content(&posts, "https://vx.gay");

        // Check mobile tiny: uses display_title and subtitle
        assert!(boxes.tiny.contains("Autonomous System: Part 1"));
        assert!(boxes.tiny.contains("» WAN Clustering Fallacy"));
        assert!(boxes.tiny.contains("[ View Post >> ]"));
        assert!(!boxes.tiny.contains(">Autonomous System: Part 1 - WAN Clustering Fallacy</a>"));

        // Check mobile small: uses display_title and subtitle
        assert!(boxes.small.contains("Autonomous System: Part 1"));
        assert!(boxes.small.contains("» WAN Clustering Fallacy"));
        assert!(boxes.small.contains("[ View Post >> ]"));

        // Check medium (first 3 posts): uses display_title and subtitle
        assert!(boxes.medium.contains("Autonomous System: Part 1"));
        assert!(boxes.medium.contains("» WAN Clustering Fallacy"));
        assert!(boxes.medium.contains("[ View Post >> ]"));
        assert!(boxes.medium.contains("Simple Post Without Subtitle"));
        // Second post has no subtitle, so no second "» "
        let count_arrows = boxes.medium.matches('»').count();
        assert_eq!(count_arrows, 1, "Only posts with subtitle should have '» '");

        // Check large
        assert!(boxes.large.contains("Autonomous System: Part 1"));
        assert!(boxes.large.contains("» WAN Clustering Fallacy"));
        assert!(boxes.large.contains("[ View Post >> ]"));
    }

    #[test]
    fn test_create_posts_content_title_wrapping() {
        let long_sub_post = Post {
            title: "Long Title That Definitely Requires Wrapping Because It Is More Than Sixty Characters Long".to_string(),
            short_title: Some("Short Title That Also Exceeds Normal Wrap Width On Narrow Containers".to_string()),
            subtitle: Some("Subtitle That Is Extraordinarily Long And Needs To Wrap Nicely Across Multiple Lines".to_string()),
            date: "2026-08-14".to_string(),
            slug: "long-post".to_string(),
            tags: vec![],
            content: "Some content here.".to_string(),
            content_html: "<p>Some content</p>".to_string(),
            href: "/post/long-post".to_string(),
            series: None,
            series_order: None,
        };

        let posts = vec![long_sub_post];
        let manager = ContentManager::new();
        let boxes = manager.create_posts_content(&posts, "https://vx.gay");

        // All boxes should render successfully without panicking and contain wrapped elements
        assert!(boxes.tiny.contains("Short Title"));
        assert!(boxes.tiny.contains("» Subtitle"));
        assert!(boxes.medium.contains("Short Title"));
        assert!(boxes.large.contains("Short Title"));
    }

    #[test]
    fn test_blog_card_content_and_header_display_title() {
        let post_with_sub = Post {
            title: "Full Title of Post".to_string(),
            short_title: Some("Short Header Title".to_string()),
            subtitle: Some("Fascinating Subtitle".to_string()),
            date: "2026-09-01".to_string(),
            slug: "full-title".to_string(),
            tags: vec![],
            content: "Excerpt of post content.".to_string(),
            content_html: "<p>Excerpt</p>".to_string(),
            href: "/post/full-title".to_string(),
            series: None,
            series_order: None,
        };
        let post_no_sub = Post {
            title: "Standalone Post".to_string(),
            short_title: None,
            subtitle: None,
            date: "2026-09-02".to_string(),
            slug: "standalone".to_string(),
            tags: vec![],
            content: "Excerpt of standalone post.".to_string(),
            content_html: "<p>Excerpt</p>".to_string(),
            href: "/post/standalone".to_string(),
            series: None,
            series_order: None,
        };

        let make_content = |post: &Post, width: usize, max_len: usize| {
            let excerpt = wrap_text(&post.content[..post.content.len().min(max_len)], width).join("\n");
            if let Some(ref sub) = post.subtitle {
                let wrapped_sub = wrap_text(&format!("» {}", sub), width).join("\n");
                format!(
                    "{}\n{}\n\n{}\n\n<a href=\"{}\">[ View Post >> ]</a>",
                    wrapped_sub, post.date, excerpt, post.href
                )
            } else {
                format!(
                    "{}\n\n{}\n\n<a href=\"{}\">[ View Post >> ]</a>",
                    post.date, excerpt, post.href
                )
            }
        };

        // Post with subtitle
        let content_sub = make_content(&post_with_sub, WRAP_WIDTH_MEDIUM, 120);
        assert!(content_sub.starts_with("» Fascinating Subtitle\n2026-09-01"));
        assert!(content_sub.contains("<a href=\"/post/full-title\">[ View Post >> ]</a>"));
        let box_header = create_header_box(post_with_sub.display_title(), &content_sub, BOX_WIDTH_MEDIUM);
        assert!(box_header.contains("Short Header Title"));

        // Post without subtitle
        let content_no_sub = make_content(&post_no_sub, WRAP_WIDTH_MEDIUM, 120);
        assert!(content_no_sub.starts_with("2026-09-02"));
        assert!(!content_no_sub.contains('»'));
        assert!(content_no_sub.contains("<a href=\"/post/standalone\">[ View Post >> ]</a>"));
        let box_header_no_sub = create_header_box(post_no_sub.display_title(), &content_no_sub, BOX_WIDTH_MEDIUM);
        assert!(box_header_no_sub.contains("Standalone Post"));
    }

    #[test]
    fn test_index_template_additional_posts() {
        let tera = tera::Tera::new("templates/**/*").expect("Failed to compile Tera templates");
        let mut ctx = tera::Context::new();

        // Create minimal context for index.html.tera
        let post = Post {
            title: "Test Additional Post".to_string(),
            short_title: None,
            subtitle: Some("Test Additional Subtitle".to_string()),
            date: "2026-09-01".to_string(),
            slug: "test-additional".to_string(),
            tags: vec![],
            content: "Test additional content.".to_string(),
            content_html: "<p>Content</p>".to_string(),
            href: "/post/test-additional".to_string(),
            series: None,
            series_order: None,
        };

        let post_with_short = Post {
            title: "Long Verbose Post Title".to_string(),
            short_title: Some("Short Title".to_string()),
            subtitle: None,
            date: "2026-09-02".to_string(),
            slug: "test-short".to_string(),
            tags: vec![],
            content: "Test short content.".to_string(),
            content_html: "<p>Content</p>".to_string(),
            href: "/post/test-short".to_string(),
            series: None,
            series_order: None,
        };

        ctx.insert("additional_posts", &vec![post, post_with_short]);
        ctx.insert("stars", &Vec::<crate::mods::Star>::new());
        ctx.insert("welcome_box", &crate::mods::BoxSizes {
            tiny: String::new(), small: String::new(), medium: String::new(), large: String::new()
        });
        ctx.insert("latest_post_box", &crate::mods::BoxSizes {
            tiny: String::new(), small: String::new(), medium: String::new(), large: String::new()
        });
        ctx.insert("footer_box", &crate::mods::BoxSizes {
            tiny: String::new(), small: String::new(), medium: String::new(), large: String::new()
        });
        ctx.insert("navigation_box", &crate::mods::BoxSizes {
            tiny: String::new(), small: String::new(), medium: String::new(), large: String::new()
        });
        ctx.insert("about_box", &crate::mods::BoxSizes {
            tiny: String::new(), small: String::new(), medium: String::new(), large: String::new()
        });
        ctx.insert("categories_box", &crate::mods::BoxSizes {
            tiny: String::new(), small: String::new(), medium: String::new(), large: String::new()
        });
        ctx.insert("title_art", "ART");
        ctx.insert("debug", &false);
        ctx.insert("canonical", "https://zoa.sh");
        ctx.insert("og_image", "https://zoa.sh/static/og-image.gif");

        let rendered = tera.render("index.html.tera", &ctx).expect("Failed to render index.html.tera");
        assert!(rendered.contains("Test Additional Post\n» Test Additional Subtitle\n2026-09-01"));
        assert!(rendered.contains("<a href=\"/post/test-additional\">[ View Post >> ]</a>"));
        assert!(rendered.contains("Short Title\n2026-09-02"));
        assert!(!rendered.contains("Long Verbose Post Title"));
    }

    #[test]
    fn test_posts_frontmatters_contain_short_title_and_subtitle() {
        let posts = crate::mods::markdown::load_all_posts("posts");
        assert!(!posts.is_empty(), "posts directory should not be empty");

        let blackwall = posts
            .iter()
            .find(|p| p.slug == "blackwall-active-defense-anycast-firewall")
            .expect("blackwall post should exist");
        assert_eq!(blackwall.short_title.as_deref(), Some("Blackwall Edge Firewall"));
        assert_eq!(
            blackwall.subtitle.as_deref(),
            Some("Building an Automated Active-Defense Firewall on Anycast Edge Nodes")
        );

        let wan = posts
            .iter()
            .find(|p| p.slug == "autonomous-system-part-1-wan-clustering-fallacy")
            .expect("wan clustering post should exist");
        assert_eq!(wan.short_title.as_deref(), Some("The WAN Clustering Fallacy"));
        assert_eq!(
            wan.subtitle.as_deref(),
            Some("The Autonomous System Architecture, Part 1")
        );

        let dashboard = posts
            .iter()
            .find(|p| p.slug == "building-financial-metrics-dashboard-in-async-rust")
            .expect("dashboard post should exist");
        assert_eq!(dashboard.short_title.as_deref(), Some("Async Rust Financial Dashboard"));
        assert_eq!(
            dashboard.subtitle.as_deref(),
            Some("Real-Time Ledger State Machines with Tokio & SQLx")
        );

        let knot = posts
            .iter()
            .find(|p| p.slug == "knot-dns-breaking-frozen-serial-trap")
            .expect("knot dns post should exist");
        assert_eq!(knot.short_title.as_deref(), Some("Knot DNS Serial Trap Recovery"));
        assert_eq!(
            knot.subtitle.as_deref(),
            Some("Automated Primary Recovery and AXFR Replication Bypass")
        );

        let postgres_split = posts
            .iter()
            .find(|p| p.slug == "postgresql-failover-hunting-dual-writer-split-brain")
            .expect("postgres split brain post should exist");
        assert_eq!(postgres_split.short_title.as_deref(), Some("Postgres Split-Brain Hunt"));
        assert_eq!(
            postgres_split.subtitle.as_deref(),
            Some("Surviving Container IP Aliasing and Standby Promotion")
        );

        let incus_wal = posts
            .iter()
            .find(|p| p.slug == "streaming-physical-replication-in-incus")
            .expect("incus wal replication post should exist");
        assert_eq!(incus_wal.short_title.as_deref(), Some("Incus Postgres WAL Streaming"));
        assert_eq!(
            incus_wal.subtitle.as_deref(),
            Some("WAL Archiving and Standby Rebuilds Without Heavy Orchestrators")
        );

        let bgp_dead_route = posts
            .iter()
            .find(|p| p.slug == "debugging-dead-route-bgp-cutover-port-80-drop")
            .expect("bgp dead route post should exist");
        assert_eq!(bgp_dead_route.short_title.as_deref(), Some("Debugging the Dead BGP Route"));
        assert_eq!(
            bgp_dead_route.subtitle.as_deref(),
            Some("Tracing Asymmetric Routing and Dummy Interface Ephemerality")
        );

        let monero_failover = posts
            .iter()
            .find(|p| p.slug == "self-hosting-monero-zero-downtime-daemon-failover")
            .expect("monero daemon failover post should exist");
        assert_eq!(monero_failover.short_title.as_deref(), Some("Monero Daemon Failover"));
        assert_eq!(
            monero_failover.subtitle.as_deref(),
            Some("Automating Payment Daemon Health Checks and Failover Routing")
        );
    }

    #[actix_web::test]
    async fn test_rss_feed_contains_categories() {
        let req = actix_web::test::TestRequest::default()
            .insert_header((actix_web::http::header::HOST, "zoa.sh"))
            .to_http_request();
        let resp = crate::rss_feed(req).await.expect("rss_feed handler failed");
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let body_bytes = actix_web::body::to_bytes(resp.into_body()).await.expect("reading body failed");
        let xml = String::from_utf8(body_bytes.to_vec()).expect("valid utf-8");

        let posts = crate::mods::markdown::load_all_posts("posts");
        let post_with_tags = posts
            .iter()
            .find(|p| !p.tags.is_empty())
            .expect("should have at least one post with tags");

        for tag in &post_with_tags.tags {
            let category_tag = format!("<category>{}</category>", tag);
            assert!(
                xml.contains(&category_tag),
                "RSS XML missing category tag '{}'",
                category_tag
            );
        }

        assert!(xml.contains("<category>"), "RSS feed should contain at least one <category> element");
        let cat_pos = xml.find("<category>").unwrap();
        let item_start = xml[..cat_pos].rfind("<item>").expect("<item> before <category>");
        let item_end = xml[item_start..].find("</item>").expect("</item> after item_start") + item_start;
        let item_xml = &xml[item_start..item_end];
        let item_cat_pos = item_xml.find("<category>").unwrap();
        let item_desc_pos = item_xml.find("<description>").expect("item should have <description>");
        assert!(
            item_cat_pos < item_desc_pos,
            "<category> should appear before <description> in <item>"
        );
    }
}

