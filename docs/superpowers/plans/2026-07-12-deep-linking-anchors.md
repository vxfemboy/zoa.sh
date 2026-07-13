# Deep-linking: Experience Bitmask + Blog Heading Anchors — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make site parts linkable — `/about` jobs auto-expand from a `#exp;<base36 bitmask>` fragment (and toggling syncs it back), and blog headings get `id` slugs + hover `#` anchors so `…/post/<slug>#section-slug` jumps to a section.

**Architecture:** Everything is URL fragments (client-side JS + server-rendered `id`/`data-bit` attributes). The server assigns each job a stable bit (alphabetical company slug) and each heading a unique slug; the browser reads them to open jobs / jump to sections. `curl`/no-JS is unaffected.

**Tech Stack:** Rust, actix-web, Tera, pulldown-cmark, syntect; vanilla JS.

## Global Constraints

- Experience fragment: `#exp` = focus block; `#exp;<base36>` = base36 of the OR'd bitmask of open jobs. JS bitwise is 32-bit → supports ≤31 jobs.
- Bits are auto-assigned by **alphabetical company slug** (stable across timeline reorders; server-side).
- Blog headings use **readable GitHub-style slugs** (`## registry vs registrar, and what EPP is` → `id="registry-vs-registrar-and-what-epp-is"`), deduped with `-2`, `-3`, … Only `h2`–`h6` get anchors (h1 is the post title).
- `curl`/no-JS unchanged: the ANSI heading must NOT show the injected `#` (strip the `.heading-anchor`); the ANSI experience view already shows all bios.
- CI: `cargo fmt --all -- --check` + `cargo clippy --all-targets --all-features -- -D warnings` clean; tests in `src/mods/tests.rs`. Build the server with `SKIP_WASM=1`.

---

### Task 1: Shared `slugify` helper

**Files:**
- Modify: `src/mods/markdown.rs` (extract `slugify`, reuse in `generate_slug`)
- Test: `src/mods/tests.rs`

**Interfaces:**
- Produces: `pub fn slugify(s: &str) -> String` — lowercase; `[a-z0-9]` kept, everything else → `-`; runs of `-` collapsed; leading/trailing `-` trimmed.

- [ ] **Step 1: Write the failing tests** — add to the `unit_tests` block in `src/mods/tests.rs`:

```rust
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
```

- [ ] **Step 2: Run it — fails**

Run: `cargo test test_slugify 2>&1 | tail -8`
Expected: FAIL — `slugify` not found in `markdown`.

- [ ] **Step 3: Add `slugify` and reuse it in `generate_slug`** — in `src/mods/markdown.rs`, add above `generate_slug`:

```rust
/// URL-safe slug: lowercase, `[a-z0-9]` kept, everything else collapsed to `-`,
/// with leading/trailing dashes trimmed.
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
```

Then replace `generate_slug`'s fallback body (the `title.to_lowercase()…join("-")` chain) with `slugify(title)`:

```rust
    // Fall back to title-based slug
    slugify(title)
}
```

- [ ] **Step 4: Run it — passes**

Run: `cargo test test_slugify 2>&1 | tail -6` → PASS. Then `cargo test 2>&1 | tail -6` → all pass.

- [ ] **Step 5: fmt + commit**

```bash
cargo fmt --all
git add src/mods/markdown.rs src/mods/tests.rs
git commit -m "markdown: extract shared slugify helper :3"
```

---

### Task 2: Blog heading anchors (markdown ids + hover `#`, ANSI clean, CSS)

**Files:**
- Modify: `src/mods/markdown.rs` (`markdown_to_html` heading interception)
- Modify: `src/mods/text.rs` (h1–h6 arm: exclude `.heading-anchor` text)
- Modify: `static/css/main.css` (heading-anchor + scroll styles)
- Test: `src/mods/tests.rs`

**Interfaces:**
- Consumes: `slugify` (Task 1).
- Produces: post HTML where `h2`–`h6` are `<hN id="slug">…<a class="heading-anchor" href="#slug" aria-label="link to this section">#</a></hN>`.

- [ ] **Step 1: Write the failing test** — in `unit_tests`:

```rust
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
    // h2 gets an id + anchor
    assert!(h.contains("<h2 id=\"foo-bar\">"));
    assert!(h.contains("class=\"heading-anchor\" href=\"#foo-bar\""));
    // duplicate heading text is deduped
    assert!(h.contains("<h2 id=\"foo-bar-2\">"));
    // h3 anchored too
    assert!(h.contains("<h3 id=\"baz\">"));
    // h1 (post title) is NOT given an anchor
    assert!(h.contains("<h1"));
    assert!(!h.contains("id=\"title\""));
    std::fs::remove_file(&p).ok();
}
```

- [ ] **Step 2: Run it — fails**

Run: `cargo test test_markdown_heading_anchors 2>&1 | tail -10`
Expected: FAIL — no `id=`/`heading-anchor` in output.

- [ ] **Step 3: Add imports + helpers** — in `src/mods/markdown.rs`, extend the pulldown import and add `HashSet`:

```rust
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashSet;
```

Add these free functions near `markdown_to_html`:

```rust
fn heading_num(level: HeadingLevel) -> u32 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Ensure a slug is unique within one document (append -2, -3, … on collision).
fn unique_slug(base: &str, used: &mut HashSet<String>) -> String {
    let base = if base.is_empty() { "section".to_string() } else { base.to_string() };
    if used.insert(base.clone()) {
        return base;
    }
    let mut n = 2;
    loop {
        let cand = format!("{base}-{n}");
        if used.insert(cand.clone()) {
            return cand;
        }
        n += 1;
    }
}
```

- [ ] **Step 4: Intercept heading events** — in `markdown_to_html`, add state after the `code_block_content` declarations:

```rust
    let mut heading: Option<HeadingLevel> = None;
    let mut heading_html = String::new();
    let mut heading_text = String::new();
    let mut used_slugs: HashSet<String> = HashSet::new();
```

Then, inside the `for event in parser { match event {` block, insert these three arms **immediately after** the existing `Event::Text(text) if code_block_lang.is_some() || !code_block_content.is_empty() =>` arm (and before the `Event::Text(text) if code_block_lang.is_none() …` arm):

```rust
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(level);
                heading_html.clear();
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                let lvl = heading.take().map(heading_num).unwrap_or(1);
                if lvl >= 2 {
                    let slug = unique_slug(&slugify(&heading_text), &mut used_slugs);
                    html_output.push_str(&format!(
                        "<h{lvl} id=\"{slug}\">{heading_html}<a class=\"heading-anchor\" href=\"#{slug}\" aria-label=\"link to this section\">#</a></h{lvl}>"
                    ));
                } else {
                    html_output.push_str(&format!("<h{lvl}>{heading_html}</h{lvl}>"));
                }
            }
            ev if heading.is_some() => {
                if let Event::Text(ref t) = ev {
                    heading_text.push_str(t);
                } else if let Event::Code(ref c) = ev {
                    heading_text.push_str(c);
                }
                push_html_event(&mut heading_html, ev);
            }
```

(The existing `Event::Text … code_block_content.is_empty()` and `other =>` arms stay unchanged after these.)

- [ ] **Step 5: Run it — passes**

Run: `cargo test test_markdown_heading_anchors 2>&1 | tail -8` → PASS. Then `cargo test 2>&1 | tail -6` → all pass.

- [ ] **Step 6: Keep the ANSI heading clean** — in `src/mods/text.rs`, the `"h1" | "h2" | … | "h6" =>` arm starts with:

```rust
            let text = element.text().collect::<String>().trim().to_string();
```

Replace that single line with a version that skips the injected anchor:

```rust
            let text = element
                .children()
                .filter_map(|n| {
                    if let Some(el) = scraper::ElementRef::wrap(n) {
                        let is_anchor = el
                            .value()
                            .attr("class")
                            .unwrap_or("")
                            .split_whitespace()
                            .any(|c| c == "heading-anchor");
                        if is_anchor {
                            None
                        } else {
                            Some(el.text().collect::<String>())
                        }
                    } else {
                        n.value().as_text().map(|t| t.to_string())
                    }
                })
                .collect::<String>()
                .trim()
                .to_string();
```

(`scraper::ElementRef` is already imported in text.rs as `ElementRef`; use the short name `ElementRef::wrap` if the import is present — check the top of the file and match it.)

- [ ] **Step 7: CSS** — append to `static/css/main.css`:

```css
/* Blog heading anchors: hover-reveal `#` link + smooth in-page jumps. */
html { scroll-behavior: smooth; }
.post-html-content h2,
.post-html-content h3,
.post-html-content h4 { scroll-margin-top: 1.2em; }
.post-html-content .heading-anchor {
    margin-left: 0.4em;
    color: #4f5b66;
    text-decoration: none;
    opacity: 0;
    transition: opacity 0.15s ease, color 0.15s ease;
}
.post-html-content h2:hover .heading-anchor,
.post-html-content h3:hover .heading-anchor,
.post-html-content h4:hover .heading-anchor,
.post-html-content .heading-anchor:focus { opacity: 1; }
.post-html-content .heading-anchor:hover { color: #33aaff; }
```

- [ ] **Step 8: Build + integration verify (anchors present, ANSI clean)**

```bash
SKIP_WASM=1 cargo build 2>&1 | tail -2
SKIP_WASM=1 ./target/debug/zoa-sh >/tmp/zt.log 2>&1 & P=$!; sleep 3
echo "-- browser HTML: heading id + anchor --"
curl -s -A Mozilla http://localhost:8084/post/namecheap-wtf | grep -o '<h2 id="registry-vs-registrar-and-what-epp-is">' | head -1
echo "-- curl ANSI: heading has NO stray # --"
curl -s -A curl http://localhost:8084/post/namecheap-wtf | grep -a 'registry vs registrar' | head -1 | cat -v | grep -o '#registry' && echo "BAD: # leaked" || echo "clean ✓"
kill $P
```
Expected: the `<h2 id=…>` prints; the ANSI check prints `clean ✓`.

- [ ] **Step 9: gate + commit**

```bash
cargo fmt --all -- --check && SKIP_WASM=1 cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -3
git add src/mods/markdown.rs src/mods/text.rs static/css/main.css src/mods/tests.rs
git commit -m "blog: heading id slugs + hover # anchors (ansi-clean) :3"
```

---

### Task 3: Experience bit map + `data-bit`

**Files:**
- Modify: `src/mods/about.rs` (bit map from company slugs; `data-bit` on `.exp-job`)
- Test: `src/mods/tests.rs`

**Interfaces:**
- Consumes: `slugify` (Task 1).
- Produces: `pub(crate) fn company_bit_map() -> std::collections::HashMap<String, u32>` — unique company slugs sorted alphabetically → bit index `0..N`. Each rendered `.exp-job` carries `data-bit="<bit>"`.

- [ ] **Step 1: Write the failing test** — in `unit_tests`:

```rust
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
```

- [ ] **Step 2: Run it — fails**

Run: `cargo test test_company_bit_map_is_stable_and_alphabetical 2>&1 | tail -8`
Expected: FAIL — `company_bit_map` not found.

- [ ] **Step 3: Add `company_bit_map` + emit `data-bit`** — in `src/mods/about.rs`, add (after `pub fn experience()`):

```rust
/// Stable per-company bit index for the `#exp;<base36 bitmask>` deep-links.
/// Unique company slugs sorted alphabetically → bit 0..N. Reordering the
/// timeline never changes a company's bit; only renaming a company does.
pub(crate) fn company_bit_map() -> std::collections::HashMap<String, u32> {
    let mut slugs: Vec<String> = experience()
        .iter()
        .map(|j| crate::mods::markdown::slugify(&j.company))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    slugs.sort();
    slugs
        .into_iter()
        .enumerate()
        .map(|(i, s)| (s, i as u32))
        .collect()
}
```

Then in `experience_box`, after `let jobs = experience();` add `let bits = company_bit_map();`, and change the job-opening line:

```rust
        s.push_str("<div class=\"exp-job\">");
```

to:

```rust
        let bit = bits[&crate::mods::markdown::slugify(&job.company)];
        s.push_str(&format!("<div class=\"exp-job\" data-bit=\"{bit}\">"));
```

- [ ] **Step 4: Run it — passes**

Run: `cargo test test_company_bit_map_is_stable_and_alphabetical 2>&1 | tail -8` → PASS. Then `cargo test 2>&1 | tail -6` → all pass.

- [ ] **Step 5: gate + commit**

```bash
cargo fmt --all -- --check && SKIP_WASM=1 cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -3
git add src/mods/about.rs src/mods/tests.rs
git commit -m "about: stable per-company bit + data-bit on jobs :3"
```

---

### Task 4: `#exp` anchor + `about.js` fragment sync

**Files:**
- Modify: `templates/about.html.tera` (wrap experience boxes in `<div id="exp">`)
- Modify: `static/js/about.js` (fragment ↔ open-state)

**Interfaces:**
- Consumes: `data-bit` on `.exp-job` (Task 3); `id="exp"` scroll target.

- [ ] **Step 1: Wrap the experience boxes** — in `templates/about.html.tera`, the two experience divs are:

```html
                <div class="ascii-box exp-desktop" data-no-responsive>
                    <div class="exp-box">{{ experience_box | safe }}</div>
                </div>
                <div class="ascii-box exp-mobile" data-no-responsive>
                    <div class="exp-box">{{ experience_box_mobile | safe }}</div>
                </div>
```

Wrap both in a single `<div id="exp">…</div>`:

```html
                <div id="exp">
                <div class="ascii-box exp-desktop" data-no-responsive>
                    <div class="exp-box">{{ experience_box | safe }}</div>
                </div>
                <div class="ascii-box exp-mobile" data-no-responsive>
                    <div class="exp-box">{{ experience_box_mobile | safe }}</div>
                </div>
                </div>
```

- [ ] **Step 2: Replace `static/js/about.js`** with the fragment-syncing version (keeps the toggle, adds URL sync + restore):

```javascript
// Experience timeline: expand/collapse jobs and mirror the open set into the URL
// fragment. Fragment is #exp (focus the block) or #exp;<base36 bitmask of the open
// jobs' data-bit values>. Bits are server-assigned (stable). curl/no-JS shows all.

function expOpenMask() {
  // OR the bits of all open jobs (dedup across desktop/mobile copies).
  let mask = 0;
  document.querySelectorAll('.exp-job.open[data-bit]').forEach((j) => {
    mask |= 1 << Number(j.dataset.bit);
  });
  return mask;
}

function expSetJob(bit, open) {
  document.querySelectorAll('.exp-job[data-bit="' + bit + '"]').forEach((node) => {
    node.classList.toggle('open', open);
    const head = node.querySelector('.exp-head');
    if (head) {
      head.setAttribute('aria-expanded', open ? 'true' : 'false');
      const t = head.querySelector('.exp-toggle');
      if (t) t.textContent = open ? '[-]' : '[+]';
    }
  });
}

function expSyncFragment() {
  const mask = expOpenMask();
  history.replaceState(null, '', mask ? '#exp;' + mask.toString(36) : '#exp');
}

document.querySelectorAll('.exp-head').forEach((head) => {
  head.addEventListener('click', () => {
    const node = head.closest('.exp-job');
    if (!node) return;
    const bit = Number(node.dataset.bit);
    expSetJob(bit, !node.classList.contains('open'));
    expSyncFragment();
  });
});

// Restore from the fragment on load, then scroll to the block.
(function expRestore() {
  const h = location.hash;
  if (!h.startsWith('#exp')) return;
  const semi = h.indexOf(';');
  if (semi !== -1) {
    const mask = parseInt(h.slice(semi + 1), 36) || 0;
    document.querySelectorAll('.exp-job[data-bit]').forEach((j) => {
      if (mask & (1 << Number(j.dataset.bit))) expSetJob(Number(j.dataset.bit), true);
    });
  }
  const target = document.getElementById('exp');
  if (target) target.scrollIntoView({ behavior: 'smooth', block: 'start' });
})();
```

- [ ] **Step 3: Build + browser verify** — start the server, open `http://localhost:8084/about` in the browser (chrome-devtools MCP):
  - Click a job → it expands AND the URL gains `#exp;<code>` (check `location.hash`).
  - Expand two → hash is a single base36 mask; copy it, reload → both reopen and the page scrolls to the experience block.
  - Visit `/about#exp` → scrolls to the block, nothing expanded.
  - Collapse all → hash becomes `#exp`.

```bash
SKIP_WASM=1 cargo build 2>&1 | tail -2
SKIP_WASM=1 ./target/debug/zoa-sh >/tmp/zt.log 2>&1 &
# (drive the checks above in the browser; then kill the server)
```

- [ ] **Step 4: commit**

```bash
git add templates/about.html.tera static/js/about.js
git commit -m "about: #exp bitmask deep-links (url <-> expanded jobs) :3"
```

---

### Task 5: Full verification (browser + curl)

**Files:** none (verification).

- [ ] **Step 1** — With the server running, verify in a real browser (chrome-devtools MCP):
  - `/about#exp;<mask>` opens the coded jobs across desktop AND mobile widths (resize to confirm state persists); toggling updates the fragment.
  - A blog post: hovering an `h2`/`h3` reveals the `#`; clicking it sets `…#slug`; loading `…/post/namecheap-wtf#registry-vs-registrar-and-what-epp-is` scrolls to that section (smoothly, with top margin).
- [ ] **Step 2** — `curl -s -A curl http://localhost:8084/post/namecheap-wtf | sed -n '/registry vs registrar/p'` shows the heading with **no** leading/trailing `#`; `curl -s -A curl http://localhost:8084/about` experience view is unchanged (all bios shown).
- [ ] **Step 3** — Final gate: `cargo fmt --all -- --check`, `SKIP_WASM=1 cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` all clean.

---

## Notes for the implementer

- `slugify` is `pub` in `markdown.rs`; reference it as `crate::mods::markdown::slugify` from `about.rs`.
- The two experience-box variants (`.exp-desktop`, `.exp-mobile`) both render the same jobs, so a company's `data-bit` is identical in both — the JS `data-bit` selectors intentionally act on both copies.
- JS bitwise is 32-bit (`1 << bit`) — fine for ≤31 jobs (there are ~13). Don't switch to BigInt.
- Keep asset/link behavior from the multi-domain work intact; this change only adds `id`/`data-bit`/anchors, no URL rewrites.
