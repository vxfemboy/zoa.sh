# Multi-domain Awareness + namecheap.wtf Vanity URL — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the site render every self-referential URL, Open Graph tag, email, and prose "mention" for the domain the visitor arrived on (zoa.sh / vx.gay / namecheap.wtf→vx.gay), add per-post OG images, and fix namecheap.wtf assets + curl-aware `:80` at the Caddy layer.

**Architecture:** A per-request `Site` (resolved from the `Host` header) carries `{domain, base_url, email}`. Handlers thread it into Tera context and the ASCII-box content builders; links/OG/email become absolute effective-domain, prose swaps only via opt-in `{domain}`/`{email}` tokens, assets stay root-relative. Caddy serves namecheap.wtf's post at its root and keeps curl on plain HTTP.

**Tech Stack:** Rust, actix-web 4, Tera, pulldown-cmark; Caddy v2 (separate `femboy/deploy` repo).

## Global Constraints

- Default / fallback / namecheap.wtf effective domain = **vx.gay**. Full domains: **zoa.sh**, **vx.gay**.
- Per-domain email: `zoa.sh → zoa@zoa.sh`, `vx.gay → z@vx.gay` (namecheap.wtf uses vx.gay's).
- Token swap is **opt-in only**: `{domain}` and `{email}` are the only tokens; every literal (esp. `github.com/vxfemboy/zoa.sh`) stays verbatim.
- Assets (`/static`, `/cat`, wasm, post images) stay **root-relative**. No changes to `cat.js` / `scroll-container.js`.
- Author identity stays `vxfemboy` on all domains.
- `time = "=0.3.47"` must not change. CI runs `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`; both must stay clean.
- Rust tests live in `src/mods/tests.rs` (a `#[cfg(test)] mod unit_tests { use crate::mods::*; ... }` block). Run a single test with `cargo test <name>`.

---

### Task 1: `Site` resolver + token substitution

**Files:**
- Create: `src/mods/site.rs`
- Modify: `src/mods.rs` (register the module + re-export)
- Test: `src/mods/tests.rs`

**Interfaces:**
- Produces:
  - `pub struct Site { pub domain: String, pub base_url: String, pub email: String }`
  - `pub fn resolve(host: &str) -> Site`
  - `impl Site { pub fn apply(&self, s: &str) -> String }` — replaces `{domain}`→domain, `{email}`→email.
  - `impl Site { pub fn og_image(&self, social: Option<&str>) -> String }` — `social` → `{base_url}/post/assets/{social}`, else `{base_url}/static/og-image.gif`.

- [ ] **Step 1: Write the failing tests** — add to the `unit_tests` module in `src/mods/tests.rs`:

```rust
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
    assert_eq!(v.apply("github.com/vxfemboy/zoa.sh"), "github.com/vxfemboy/zoa.sh");
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_site_ 2>&1 | tail -20`
Expected: FAIL — `use of undeclared crate or module site` / `site::resolve` not found.

- [ ] **Step 3: Create `src/mods/site.rs`**

```rust
//! Per-request effective domain. The site is served on zoa.sh, vx.gay, and
//! namecheap.wtf (a vanity URL for one post). Every self-referential URL, OG
//! tag, email, and `{domain}`/`{email}` token is rendered for the domain the
//! visitor arrived on. namecheap.wtf and any unknown host default to vx.gay.

/// The resolved identity for one request.
pub struct Site {
    pub domain: String,
    pub base_url: String,
    pub email: String,
}

impl Site {
    fn new(domain: &str, email: &str) -> Self {
        Self {
            domain: domain.to_string(),
            base_url: format!("https://{domain}"),
            email: email.to_string(),
        }
    }

    /// Substitute the opt-in tokens. Only `{domain}` and `{email}` swap; every
    /// other character (including literal domain names) is left untouched.
    pub fn apply(&self, s: &str) -> String {
        s.replace("{domain}", &self.domain)
            .replace("{email}", &self.email)
    }

    /// Absolute OG/social image URL. `social` is a path under `posts/assets/`
    /// (frontmatter `social:`); absent → the default site image.
    pub fn og_image(&self, social: Option<&str>) -> String {
        match social {
            Some(path) => format!("{}/post/assets/{}", self.base_url, path.trim_start_matches('/')),
            None => format!("{}/static/og-image.gif", self.base_url),
        }
    }
}

/// Resolve the effective site from a request `Host` header value.
pub fn resolve(host: &str) -> Site {
    let host = host.split(':').next().unwrap_or(host).trim().to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    match host {
        "zoa.sh" => Site::new("zoa.sh", "zoa@zoa.sh"),
        // vx.gay, namecheap.wtf (defers), and anything unknown → vx.gay.
        _ => Site::new("vx.gay", "z@vx.gay"),
    }
}
```

- [ ] **Step 4: Register the module** — in `src/mods.rs`, add `pub mod site;` alphabetically (after `pub mod responsive;`) and add `pub use site::Site;` near the other re-exports. Because `tests.rs` does `use crate::mods::*;`, the tests reference the module as `site::resolve` — keep the `pub mod site;` (do NOT glob-re-export its free functions, to avoid a name clash with any future `resolve`).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test test_site_ 2>&1 | tail -20`
Expected: PASS (4 tests).

- [ ] **Step 6: fmt + commit**

```bash
cargo fmt --all
git add src/mods/site.rs src/mods.rs src/mods/tests.rs
git commit -m "site: per-request effective domain resolver + tokens :3"
```

---

### Task 2: Per-post `social` frontmatter + post image paths → root-absolute

**Files:**
- Modify: `src/mods/markdown.rs` (struct field, frontmatter parse, image rewrite)
- Test: `src/mods/tests.rs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `MarkdownPost` gains `pub social: Option<String>`. `markdown_to_html` output has post image `src="assets/…"` rewritten to `src="/post/assets/…"`.

- [ ] **Step 1: Write the failing tests** — add to `unit_tests` in `src/mods/tests.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_markdown_rewrites_relative_asset_images 2>&1 | tail -20`
Expected: FAIL — no field `social` on `MarkdownPost`.

- [ ] **Step 3: Add the `social` field** — in `src/mods/markdown.rs`, add to `struct MarkdownPost` after `content_plain`:

```rust
    /// Optional per-post OG/social image, a path under `posts/assets/`
    /// (frontmatter `social:`). None → the default site image.
    pub social: Option<String>,
```

- [ ] **Step 4: Parse it + build the field** — in `load_markdown_file`, before `Some(MarkdownPost {` add:

```rust
    let social = frontmatter
        .get("social")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
```

and add `social,` to the `MarkdownPost { … }` struct literal.

- [ ] **Step 5: Rewrite relative image srcs** — in `markdown_to_html`, change the final line `colorize_comments(&html_output)` to:

```rust
    let html = colorize_comments(&html_output);
    // Post images are authored `src="assets/…"` (relative). Make them
    // root-absolute so they load whether the post is at /post/<slug> or served
    // at a vanity domain's root (namecheap.wtf).
    html.replace("src=\"assets/", "src=\"/post/assets/")
        .replace("src='assets/", "src='/post/assets/")
}
```

(remove the old trailing `colorize_comments(&html_output)` expression and its `}` is replaced by the block above's closing `}`.)

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test test_markdown_rewrites_relative_asset_images 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 7: Full test run (nothing else broke)**

Run: `cargo test 2>&1 | tail -15`
Expected: all PASS.

- [ ] **Step 8: fmt + commit**

```bash
cargo fmt --all
git add src/mods/markdown.rs src/mods/tests.rs
git commit -m "posts: social: frontmatter + root-absolute image srcs :3"
```

---

### Task 3: Thread `base_url` into the shared boxes (nav, footer, latest-posts)

**Files:**
- Modify: `src/mods/responsive.rs` (`new_navigation`), `src/mods/ascii_art.rs` (`create_nav_box`), `src/mods/content.rs` (`create_page_context`, `create_posts_content`)
- Modify: `src/main.rs` (every `create_page_context()` call site)

**Interfaces:**
- Consumes: `Site` (Task 1) — callers pass `&site.base_url`.
- Produces: `ContentManager::create_page_context(&self, base_url: &str)`; nav/footer/post links are absolute (`{base_url}/…`).

- [ ] **Step 1: Make `create_nav_box` take a base_url** — in `src/mods/ascii_art.rs`, find `pub fn create_nav_box(items: &[NavItem], width: usize)` (~line 643). Change the signature to `pub fn create_nav_box(items: &[NavItem], width: usize, base_url: &str)`. In the body where each item builds `<a href="{href}">…`, prefix the href with `base_url` **only when it starts with `/`** (leave `#`/external hrefs alone):

```rust
    let nav_items = items
        .iter()
        .map(|item| {
            let href = if item.href.starts_with('/') {
                format!("{}{}", base_url, item.href)
            } else {
                item.href.clone()
            };
            format!("<a href=\"{}\">{}</a>", href, item.text)
        })
        .collect::<Vec<_>>();
```

(Match the exact existing string format; only the href is prefixed.)

- [ ] **Step 2: Thread through `new_navigation`** — in `src/mods/responsive.rs` (~line 67) change `pub fn new_navigation(items: &[NavItem])` to `pub fn new_navigation(items: &[NavItem], base_url: &str)` and pass `base_url` into each `create_nav_box(..., base_url)` call inside it.

- [ ] **Step 3: Make posts "Read more" links absolute** — in `src/mods/content.rs`, change `fn create_posts_content(&self, posts: &[Post])` to also take `base_url: &str`, and where each `href="{}"` uses `latest_post.href` / a post href, wrap with base_url: build `let href = format!("{}{}", base_url, latest_post.href);` and use `href` in the `format!`. Do this for every `Read more:` / `Read >>` link in that function (tiny/small/medium/large variants).

- [ ] **Step 4: Thread `base_url` into `create_page_context`** — change its signature to `pub fn create_page_context(&self, base_url: &str)`, pass `base_url` to `ResponsiveBoxes::new_navigation(&self.data.nav_items, base_url)` and to `self.create_posts_content(&processed_posts, base_url)`.

- [ ] **Step 5: Fix every call site in `src/main.rs`** — each handler resolves the site first, then calls `create_page_context(&site.base_url)`. For a handler that currently starts:

```rust
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context()?;
```

change to (using `req` — add `req: HttpRequest` as the FIRST param of every handler that renders a page: `index`, `about`, `projects`, `uses`, `now`, `blog_index`, `post_view`; import `actix_web::HttpRequest`):

```rust
    let host = req.connection_info().host().to_string();
    let site = mods::site::resolve(&host);
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context(&site.base_url)?;
```

The `index` handler uses `TemplateContextBuilder`; it still needs `site` (Task 5) — add the same 3 lines. Keep `site` in scope in every handler for Task 5.

- [ ] **Step 6: Build + integration check**

Run: `SKIP_WASM=1 cargo build 2>&1 | tail -3` → Finished.
Run the server and check nav hrefs differ by Host:

```bash
SKIP_WASM=1 ./target/debug/zoa-sh >/tmp/zt.log 2>&1 &  P=$!; sleep 3
curl -s -H 'Host: vx.gay'  http://localhost:8084/ | grep -o 'href="https://vx.gay/about"' | head -1
curl -s -H 'Host: zoa.sh'  http://localhost:8084/ | grep -o 'href="https://zoa.sh/about"' | head -1
kill $P
```

Expected: each grep prints its match (nav is absolute per domain).

- [ ] **Step 7: fmt + commit**

```bash
cargo fmt --all
git add src/mods/responsive.rs src/mods/ascii_art.rs src/mods/content.rs src/main.rs
git commit -m "boxes: absolute nav/post links per effective domain :3"
```

---

### Task 4: Domain-aware ASCII-box content (about / now / projects / data)

**Files:**
- Modify: `src/mods/about.rs` (`build_about_boxes`), `src/mods/now.rs` (`now_boxes`), `src/mods/projects.rs` (its box builder), `src/mods/data.rs` (email/domain literals → tokens)
- Modify: `src/main.rs` (pass `&site` into these builders)
- Test: `src/mods/tests.rs`

**Interfaces:**
- Consumes: `Site` (Task 1).
- Produces: `build_about_boxes(site: &Site)`, `now_boxes(site: &Site)`, and the projects builder `projects_boxes(site: &Site)` (match the real current name) — each substitutes `{domain}`/`{email}` before building boxes.

- [ ] **Step 1: Tokenize the content strings** — replace the literal self-references with tokens (leave the repo link literal):
  - `src/mods/about.rs`: the LINKS box line `• email: <a href=\"mailto:zoa@zoa.sh\">zoa@zoa.sh</a>` → `• email: <a href=\"mailto:{email}\">{email}</a>`.
  - `src/mods/now.rs`: `→ <a href=\"mailto:zoa@zoa.sh\">zoa@zoa.sh</a>` → `{email}` form; and `"zoa.sh -- this site."` → `"{domain} -- this site."`.
  - `src/mods/projects.rs`: `curl zoa.sh` (both occurrences) → `curl {domain}`; leave `github.com/vxfemboy/zoa.sh` literal.

- [ ] **Step 2: Write the failing test** — in `unit_tests`:

```rust
#[test]
fn test_about_boxes_email_swaps_by_site() {
    let v = site::resolve("vx.gay");
    let boxes = about::build_about_boxes(&v);
    assert!(boxes.links_box.large.contains("z@vx.gay"));
    assert!(!boxes.links_box.large.contains("zoa@zoa.sh"));
    // repo link literal untouched
    assert!(boxes.links_box.large.contains("github.com/vxfemboy"));
}
```

- [ ] **Step 2b: Run it — fails** (`build_about_boxes` takes no args yet).

Run: `cargo test test_about_boxes_email_swaps_by_site 2>&1 | tail -12` → FAIL.

- [ ] **Step 3: Make `build_about_boxes` site-aware** — change `pub fn build_about_boxes()` to `pub fn build_about_boxes(site: &Site)` (add `use crate::mods::Site;`). At the top, after the `links`/`summary`/`skills` strings are assembled, substitute tokens before constructing the boxes:

```rust
    let summary = site.apply(&summary);
    let skills = site.apply(&skills);
    let links = site.apply(&links);
```

Substituting the raw strings *before* `section(...)` keeps the ASCII box width/border math aligned to the real (post-swap) text.

- [ ] **Step 4: Same for `now_boxes` and `project_boxes`** — the real names are `mods::now::now_boxes()` and `mods::projects::project_boxes()` (both return `Vec<BoxSizes>`). Change each to `pub fn now_boxes(site: &Site)` / `pub fn project_boxes(site: &Site)` and `site.apply(...)` each content string before building its box. `mods::uses::uses_boxes()` has no domain/email literals — leave it argless.

- [ ] **Step 5: `data.rs`** — `SiteData::new()` builds `nav_items` (hrefs stay relative — Task 3 prefixes them) and `footer_text` (`🄯 vxfemboy | meow <3`, no domain — leave as-is). Grep `data.rs` for `zoa.sh`; there is none today — no change expected.

- [ ] **Step 6: Pass `&site` at the call sites in `src/main.rs`** — the `about` handler: `build_about_boxes(&site)`. The `projects` handler (its own fn, ~line 414): `project_boxes(&site)`. The `now` handler passes `now_boxes(&site)` into `card_page`. Give the shared `card_page(tera, config, template, key, boxes)` helper two new params — `site: &Site` and `path: &str` — so it can insert `base_url`/`canonical`/`og_image` (Task 5); the `uses`/`now` handlers resolve `site` and pass it plus their path (`"/uses"` / `"/now"`).

- [ ] **Step 7: Run the test — passes**

Run: `cargo test test_about_boxes_email_swaps_by_site 2>&1 | tail -12` → PASS.

- [ ] **Step 8: fmt + commit**

```bash
cargo fmt --all
git add src/mods/about.rs src/mods/now.rs src/mods/projects.rs src/mods/data.rs src/main.rs src/mods/tests.rs
git commit -m "content: {domain}/{email} tokens swap per effective domain :3"
```

---

### Task 5: Domain-aware templates (OG / canonical / base_url) + handler context

**Files:**
- Modify: `src/main.rs` (insert `base_url`, `domain`, `og_image`, `canonical` into each handler's Tera context; `index` via `TemplateContextBuilder`)
- Modify: `src/mods/template_builder.rs` (add `with_site`)
- Modify: `templates/index.html.tera`, `about.html.tera`, `projects.html.tera`, `uses.html.tera`, `now.html.tera`, `blog.html.tera`, `post.html.tera` (OG/canonical/twitter)

**Interfaces:**
- Consumes: `Site` (Task 1), `MarkdownPost.social` (Task 2), `Site::og_image` (Task 1).
- Produces: every template reads `{{ base_url }}`, `{{ canonical }}`, `{{ og_image }}`.

- [ ] **Step 1: `TemplateContextBuilder::with_site`** — in `template_builder.rs` add:

```rust
    pub fn with_site(mut self, base_url: &str, canonical: &str, og_image: &str) -> Self {
        self.context.insert("base_url", base_url);
        self.context.insert("canonical", canonical);
        self.context.insert("og_image", og_image);
        self
    }
```

- [ ] **Step 2: `index` handler** — after resolving `site` (Task 3), add `.with_site(&site.base_url, &site.base_url, &site.og_image(None))` to the builder chain (canonical for `/` is just `base_url`, i.e. `https://vx.gay`; and append `/` in the template).

- [ ] **Step 3: Non-index page handlers** — `about`, `projects`, `blog_index` each build a `tera::Context` directly; insert the three keys with the page's own path:

```rust
    ctx.insert("base_url", &site.base_url);
    ctx.insert("canonical", &format!("{}{}", site.base_url, "/about")); // "/projects", "/blog"
    ctx.insert("og_image", &site.og_image(None));
```

`uses` + `now` go through `card_page` — insert the same three keys **inside `card_page`** using its new `site`/`path` params (from Task 4 Step 6): `ctx.insert("canonical", &format!("{}{}", site.base_url, path));`. So the per-page path arrives via `card_page(..., path)`.

- [ ] **Step 4: `post_view` handler** — insert:

```rust
    ctx.insert("base_url", &site.base_url);
    ctx.insert("canonical", &format!("{}/post/{}", site.base_url, post.slug));
    ctx.insert("og_image", &site.og_image(post.social.as_deref()));
```

- [ ] **Step 5: Edit each template head** — in every `templates/*.tera`, replace the hardcoded OG/twitter block. Pattern (post.html.tera shown; apply the analogous change to each, using that page's existing `og:title`/`og:description` text unchanged):

```html
    <meta property="og:url" content="{{ canonical }}">
    <meta property="og:title" content="{{ title }}">
    <meta property="og:description" content="{{ title }} - A blog post by vxfemboy">
    <meta property="og:image" content="{{ og_image }}">
    ...
    <meta name="twitter:image" content="{{ og_image }}">
```

For the non-post templates, the `og:url` becomes `{{ canonical }}` and both image tags become `{{ og_image }}`. Leave `og:title`/`og:description`/`twitter:card` text as they are.

- [ ] **Step 6: Build + integration check (OG swaps by Host; per-post image; default gif)**

```bash
SKIP_WASM=1 cargo build 2>&1 | tail -2
SKIP_WASM=1 ./target/debug/zoa-sh >/tmp/zt.log 2>&1 &  P=$!; sleep 3
echo "-- home og:url per domain --"
curl -s -H 'Host: vx.gay' http://localhost:8084/ | grep -o 'og:url" content="https://vx.gay/"'
curl -s -H 'Host: zoa.sh' http://localhost:8084/ | grep -o 'og:url" content="https://zoa.sh/"'
echo "-- default og image is the gif --"
curl -s -H 'Host: vx.gay' http://localhost:8084/about | grep -o 'og:image" content="https://vx.gay/static/og-image.gif"'
echo "-- namecheap post uses its social.png, host defers to vx.gay --"
curl -s -H 'Host: namecheap.wtf' http://localhost:8084/post/namecheap-wtf | grep -o 'og:image" content="https://vx.gay/post/assets/namecheap/social.png"'
kill $P
```

Expected: every grep prints its match. (The namecheap `social.png` line requires Task 6's frontmatter; if run before Task 6 it shows the gif — fine, re-verify after Task 6.)

- [ ] **Step 7: fmt + commit**

```bash
cargo fmt --all
git add src/main.rs src/mods/template_builder.rs templates/
git commit -m "templates: OG/canonical/base_url per effective domain :3"
```

---

### Task 6: RSS / sitemap use `base_url`; namecheap post `social:` frontmatter

**Files:**
- Modify: `src/main.rs` (`rss_feed`, `sitemap`)
- Modify: `posts/namecheap.md` (frontmatter)

**Interfaces:**
- Consumes: `Site` (Task 1).

- [ ] **Step 1: `rss_feed` + `sitemap` take the request host** — add `req: HttpRequest` as the first param of each, resolve `site`, and replace every hardcoded `https://zoa.sh` in their format strings with `{}` fed by `site.base_url`. (Both functions currently take no args; update their route registrations if the signature changes — actix injects `HttpRequest` automatically, no route change needed.)

Example for one RSS line:
```rust
    // was: <link>https://zoa.sh/post/{}</link>
    format!("<link>{}/post/{}</link>", site.base_url, post.slug)
```
Apply to all `https://zoa.sh` occurrences in `rss_feed` (the channel `<link>`, `<atom:link>`, each item link/guid) and in `sitemap` (the `<loc>` lines and the `Sitemap:`/robots line — check `robots_txt` too; if it hardcodes zoa.sh, give it the same treatment).

- [ ] **Step 2: Add `social:` to the namecheap post** — in `posts/namecheap.md` frontmatter, after `slug: namecheap-wtf`, add:

```yaml
social: namecheap/social.png
```

- [ ] **Step 3: Build + verify**

```bash
SKIP_WASM=1 cargo build 2>&1 | tail -2
SKIP_WASM=1 ./target/debug/zoa-sh >/tmp/zt.log 2>&1 &  P=$!; sleep 3
curl -s -H 'Host: vx.gay' http://localhost:8084/rss.xml | grep -o 'https://vx.gay/post/' | head -1
curl -s -H 'Host: namecheap.wtf' http://localhost:8084/post/namecheap-wtf | grep -o 'https://vx.gay/post/assets/namecheap/social.png' | head -1
kill $P
```
Expected: both greps match. (`posts/assets/namecheap/social.png` is added by the user; a missing file only means the social preview 404s, not a server error.)

- [ ] **Step 4: fmt + full gate + commit**

```bash
cargo fmt --all -- --check && echo "fmt OK"
SKIP_WASM=1 cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -3
cargo test 2>&1 | tail -8
git add src/main.rs posts/namecheap.md
git commit -m "feeds: rss/sitemap per domain; namecheap social image :3"
```

---

### Task 7: Verify with the `verify` skill (real browser, three hosts)

**Files:** none (verification).

- [ ] **Step 1** — Start the server, then in the browser (chrome-devtools MCP) load `http://localhost:8084/` three times sending `Host: zoa.sh`, `Host: vx.gay`, and `/post/namecheap-wtf` with `Host: namecheap.wtf` (use `curl` for the header-driven checks; use the browser to confirm the page renders and CSS/images load). Confirm:
  - zoa.sh: nav/OG/email all `zoa.sh` / `zoa@zoa.sh`.
  - vx.gay: all `vx.gay` / `z@vx.gay`; `/projects` shows `curl vx.gay`; `/about` LINKS shows `z@vx.gay`; repo link still `github.com/vxfemboy/zoa.sh`.
  - namecheap post: nav hrefs → `https://vx.gay/<page>`; og:image = social.png; images `/post/assets/...` load.
- [ ] **Step 2** — `curl -s http://localhost:8084/ -A curl | head` still renders the ANSI site (curl path unaffected).

---

### Task 8: Caddy — curl-aware `:80` + namecheap.wtf serve-in-place, deploy

**Files:**
- Modify: `~/projects/femboy/deploy/servers/home/incus/caddy/Caddyfile` (public-web blocks only)

**Interfaces:** none (edge config).

- [ ] **Step 1: Replace the public-web blocks** — in the Caddyfile, replace the four current `# --- Public web` blocks with:

```caddy
# --- Public web: vx.gay + zoa.sh -> zoa.incus:8084; namecheap.wtf serves /post/namecheap-wtf
#     at its root. :80 serves curl/wget on plain HTTP, upgrades browsers to :443.
http://vx.gay, http://zoa.sh {
	@cli header_regexp User-Agent (?i)(curl|wget)
	handle @cli { reverse_proxy zoa.incus:8084 }
	handle {
		header Vary User-Agent
		redir https://{host}{uri}
	}
}
http://namecheap.wtf {
	@cli header_regexp User-Agent (?i)(curl|wget)
	handle @cli {
		rewrite / /post/namecheap-wtf
		reverse_proxy zoa.incus:8084
	}
	handle {
		header Vary User-Agent
		redir https://{host}{uri}
	}
}
vx.gay, zoa.sh {
	import dns01
	reverse_proxy zoa.incus:8084
}
namecheap.wtf {
	import dns01
	@root path /
	rewrite @root /post/namecheap-wtf
	reverse_proxy zoa.incus:8084
}
```

- [ ] **Step 2: Validate + deploy** — copy the Caddyfile into the `caddy` container and reload. Use the repo's existing deploy path (check `servers/home/incus/caddy/` for a push script; otherwise):

```bash
incus file push ~/projects/femboy/deploy/servers/home/incus/caddy/Caddyfile caddy/etc/caddy/Caddyfile
incus exec caddy -- caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
incus exec caddy -- sh -c 'systemctl reload caddy || rc-service caddy reload || caddy reload --config /etc/caddy/Caddyfile'
```

- [ ] **Step 3: Verify at the edge** (from the host, over the anycast/mesh — or via `incus exec caddy -- curl`):

```bash
# curl stays on HTTP (200, no redirect); browser UA upgrades (30x -> https)
curl -sI  http://zoa.sh            | head -1     # expect 200
curl -sI  http://zoa.sh -A Mozilla | head -1     # expect 302 -> https
# namecheap.wtf: page is the post; CSS is real CSS, not the post HTML
curl -s  https://namecheap.wtf/ | grep -o '<title>[^<]*'          # post title
curl -sI https://namecheap.wtf/static/css/main.css | grep -i content-type   # text/css
```
Expected: 200 for curl, 302 for browser UA; namecheap root = the post; `/static/css/main.css` = `text/css` (not HTML).

- [ ] **Step 4: Commit (deploy repo)**

```bash
cd ~/projects/femboy/deploy
git add servers/home/incus/caddy/Caddyfile
git commit -m "caddy: curl-aware :80 + namecheap.wtf serves the post at root"
```

---

### Task 9: Deploy the app + final sign-off

- [ ] **Step 1: Deploy the app** — `cd ~/projects/zoa/zoa.sh && ./deploy.sh` (build → push → restart the `zoa` container; health check green).
- [ ] **Step 2: Live sanity** — from a browser: visit `https://vx.gay/about` (email `z@vx.gay`), `https://zoa.sh/about` (email `zoa@zoa.sh`), `https://namecheap.wtf/` (the post renders with CSS + images; nav goes to vx.gay). `curl zoa.sh` renders the ANSI site over HTTP.
- [ ] **Step 3: Commit any post asset** — ensure `posts/assets/namecheap/social.png` exists (user-provided); if added, it ships on the next `./deploy.sh`.

---

## Notes for the implementer

- Confirmed current names/signatures: `mods::now::now_boxes()`, `mods::uses::uses_boxes()`, `mods::projects::project_boxes()` (all `-> Vec<BoxSizes>`); `card_page(tera, config, template, key, boxes)`; `projects` is its own handler (~line 414). Match string literals exactly when editing.
- `req.connection_info().host()` returns a `Ref`; bind it to an owned `String` immediately (`let host = req.connection_info().host().to_string();`) before further borrows.
- Keep asset URLs (`/static`, `/cat`, wasm) root-relative everywhere. Only *navigation links*, *OG/canonical*, *email*, and *`{domain}`/`{email}` tokens* become absolute/effective-domain.
- Run `cargo fmt --all` before every commit; the final gate is `fmt --check` + `clippy -D warnings` + `cargo test`.
