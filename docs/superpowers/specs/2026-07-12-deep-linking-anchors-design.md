# Deep-linking: expandable experience state + blog heading anchors — design

**Date:** 2026-07-12
**Repo:** `zoa.sh`

## Goal

Let people link to specific parts of the site:

1. **`/about` experience** — sharing/opening a URL fragment auto-expands one or many
   jobs and scrolls to the experience block; toggling jobs updates the fragment live.
2. **Blog posts** — every section heading is a linkable anchor, so
   `…/post/namecheap-wtf#registry-vs-registrar-and-what-epp-is` jumps to that section.

Both are **URL fragments** (`#…`), so everything is client-side: no server routes, no
reloads, and the `curl`/no-JS view is unaffected (the ANSI experience already shows all
bios; heading `id`s are inert in the terminal).

## Part A — `/about` experience: bitmask deep-links

### Encoding

Each job owns one **bit**. The fragment carries the base36 of the OR'd bitmask of the
open jobs:

- `#exp` — focus the experience block (scroll to it), nothing expanded.
- `#exp;w` — one job open (base36 of that job's single-bit value; e.g. bit 5 → 32 → `w`).
- `#exp;5` — bits 0 + 2 (value 5) open.
- `#exp;6b7` — all 13 open (2¹³−1 = 8191 → `6b7`).

JS encodes with `mask.toString(36)` and decodes with `parseInt(str, 36)`. Bitwise `|`/`<<`
are 32-bit in JS, so this supports up to **31 jobs** (ample; note it as a ceiling).

### Bit assignment (stable, automatic)

The server assigns each job a bit index by a **stable, deterministic key**: slugify each
company name, sort the unique slugs **alphabetically**, and the bit index is the slug's
position in that sorted list. This is fully automatic and does **not** shift when you
reorder your timeline (display stays newest-first). It changes only if you *rename* a
company, or *insert* a company whose slug sorts earlier (nudging later bits — a rare,
accepted tradeoff). Two jobs at the same company would collide on slug; today all
companies are unique — if that changes, disambiguate the bit key with the date.

### Server (`src/mods/about.rs`)

- Compute `company_slug` for each job (reuse a slugify helper) and the sorted bit map.
- Emit `data-bit="N"` on each `.exp-job` element (both the desktop and mobile variants
  get the **same** bit for the same company, since the key is the slug).

### Template (`templates/about.html.tera`)

- Wrap the two experience-box variants (`.exp-desktop` + `.exp-mobile`) in a single
  `<div id="exp">` (the `#exp` scroll target — one element, so the `id` stays unique).

### Client (`static/js/about.js`)

- **On load** — read `location.hash`:
  - `#exp` → smooth-scroll to `#exp`.
  - `#exp;<base36>` → `mask = parseInt(base36, 36)`; for every `.exp-job[data-bit]` whose
    bit is set in `mask`, open it (add `.open`, swap `[+]`→`[-]`, `aria-expanded`);
    then scroll to `#exp` (or the first opened job).
  - Because desktop + mobile variants each have a copy per company, act on **all**
    `.exp-job` matching a set bit so state survives a viewport resize.
- **On toggle** (existing click handler) — after toggling, recompute the mask from the
  set of currently-open bits (dedup across the two variants) and
  `history.replaceState(null, "", "#exp;" + mask.toString(36))` (or `#exp` when the mask
  is 0). `replaceState` so expand/collapse doesn't spam the back button.

## Part B — blog post heading anchors

### Server (`src/mods/markdown.rs`)

In the `markdown_to_html` event loop, intercept heading events (like code blocks are
intercepted today): buffer the heading's text between `Start(Heading)` and `End(Heading)`,
compute a **GitHub-style slug** (lowercase; keep `[a-z0-9]`; spaces/other → `-`; collapse
and trim `-`), dedup against a `HashSet` of used slugs (append `-2`, `-3`, …), and emit:

```html
<h2 id="registry-vs-registrar-and-what-epp-is">registry vs registrar, and what EPP is<a class="heading-anchor" href="#registry-vs-registrar-and-what-epp-is" aria-label="link to this section">#</a></h2>
```

A small `slugify` + heading-rendering helper; applies to `h2`–`h6` (h1 is the post title).

### Curl/ANSI (`src/mods/text.rs`)

The injected `#` anchor would otherwise appear as a literal `#` before each heading in the
terminal view. Add `heading-anchor` to `format_element`'s skip set so the ANSI heading
stays clean; the `id` attribute is already ignored there.

### CSS (`static/css/main.css`)

- `html { scroll-behavior: smooth; }` — smooth jumps.
- `.post-html-content h2, .post-html-content h3 { scroll-margin-top: 1.2em; position: relative; }`
  — headings don't jam against the viewport top on jump.
- `.heading-anchor { opacity: 0; margin-left: .4em; color: #4f5b66; text-decoration: none; transition: opacity .15s; }`
  and reveal on hover: `h2:hover .heading-anchor, h3:hover .heading-anchor, .heading-anchor:focus { opacity: 1; }`
  `.heading-anchor:hover { color: #33aaff; }`

## Files touched

- `src/mods/about.rs` — slugify + alphabetical bit map; `data-bit` on each `.exp-job`.
- `templates/about.html.tera` — `id="exp"` on the experience wrapper.
- `static/js/about.js` — fragment ↔ open-state sync (load + toggle), base36 bitmask,
  both-variant syncing, scroll.
- `src/mods/markdown.rs` — heading `id` slugs + hover `#` anchor injection (+ `slugify`).
- `src/mods/text.rs` — skip `.heading-anchor` in the ANSI path.
- `static/css/main.css` — heading-anchor styles + scroll-margin + smooth scroll.

## Testing

- **Unit** (`src/mods/tests.rs`): heading slugify (text → slug) + dedup (`## A` twice →
  `a`, `a-2`); the about.rs bit map (companies → stable alphabetical bit indices;
  `data-bit` present on rendered jobs).
- **Manual/browser**:
  - `/about#exp` scrolls to the block; `/about#exp;<code>` opens the coded job(s) + scrolls;
    toggling a job updates the fragment; opening several then reloading restores them.
  - Blog `…#heading-slug` jumps to the section; hovering a heading reveals the `#` link;
    clicking it sets the fragment.
  - `curl …/post/namecheap-wtf` headings render clean (no stray `#`); the experience ANSI
    view is unchanged.

## Out of scope / notes

- Base36 bitmask is `/about` only; blog headings use readable slugs.
- No persistence/registry file — the bit map is derived at render from the job list.
- Fragments (not paths); no new server routes. `curl`/no-JS behavior unchanged.
