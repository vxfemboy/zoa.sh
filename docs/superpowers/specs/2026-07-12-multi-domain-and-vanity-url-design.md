# Multi-domain awareness + namecheap.wtf vanity URL — design

**Date:** 2026-07-12
**Repos:** `zoa.sh` (the app) + `femboy/deploy` (the `caddy` container Caddyfile)

## Goal

The site is served on three domains through the `caddy` container (all reverse-proxied
to `zoa.incus:8084`):

- **zoa.sh** — full domain; the whole site self-references zoa.sh.
- **vx.gay** — full domain; the whole site self-references vx.gay. **This is the
  primary/default** (used for unknown hosts and as the canonical fallback).
- **namecheap.wtf** — a vanity URL for one blog post. It shows the post *at its own
  URL* (`namecheap.wtf/`), but the site's home for it is **vx.gay** — nav, OG, email,
  and links all point at vx.gay, never namecheap.wtf.

Make the app **domain-aware** so every self-referential URL, Open Graph tag, email,
and prose mention reflects the domain the visitor arrived on, and fix two edge issues:
namecheap.wtf's broken CSS/assets, and `:80` not upgrading browsers to HTTPS (while
keeping `curl` on plain HTTP).

## Domain model

An **effective domain** is resolved per request from the `Host` header:

| Request Host    | effective domain | base_url          | email          |
|-----------------|------------------|-------------------|----------------|
| `zoa.sh`        | zoa.sh           | `https://zoa.sh`  | `zoa@zoa.sh`   |
| `vx.gay`        | vx.gay           | `https://vx.gay`  | `z@vx.gay`     |
| `namecheap.wtf` | **vx.gay**       | `https://vx.gay`  | `z@vx.gay`     |
| anything else   | **vx.gay** (default) | `https://vx.gay` | `z@vx.gay`  |

namecheap.wtf is identical to vx.gay for *all rendering*; the only difference is that
Caddy serves the post at its root URL (below). Host is normalized: strip port,
lowercase, strip a leading `www.`.

**Host trust:** Caddy `reverse_proxy` preserves the original `Host` header upstream by
default, so the app reads the real domain from `req.connection_info().host()`.
Verification step confirms this; if Caddy is found to rewrite Host, add
`header_up Host {host}` to the proxy blocks.

## App changes (`zoa.sh`)

### 1. Effective-domain resolver

New small module (e.g. `src/mods/site.rs`) exposing:

```rust
pub struct Site { pub domain: String, pub base_url: String, pub email: String }
pub fn resolve(host: &str) -> Site   // maps per the table above; default = vx.gay
```

Handlers read the host once and build a `Site`, then pass `base_url`/`domain`/`email`
into template context and the content builders.

### 2. Domain-aware templates

Every `templates/*.tera` currently hardcodes `https://zoa.sh` and points `og:image` at
`static/og-image.png` (which does not exist). Replace with context variables:

- `og:url` / canonical → `{{ base_url }}{{ path }}` (each page passes its own path).
- `og:image` / `twitter:image` → `{{ og_image }}` (absolute; computed per page — see §4).
- Nav links, "back to blog", "read more", footer links → **absolute** `{{ base_url }}` +
  path. Uniformly absolute so namecheap.wtf's nav lands on vx.gay; on zoa.sh/vx.gay it's
  the same domain, no behavior change.

Inject `base_url`, `domain`, `email`, `og_image`, and the page path into each handler's
Tera context.

### 3. Token substitution in ASCII-box content

Server-rendered ASCII-box content (`about.rs`, `now.rs`, `projects.rs`) may contain
two **opt-in tokens**:

- `{domain}` → the effective domain (e.g. `curl {domain}`, `{domain} — this site`).
- `{email}` → the effective email (e.g. about LINKS `email: {email}`).

Substitution is **opt-in**: only these tokens swap; every literal (notably the repo
link `github.com/vxfemboy/zoa.sh`) stays exactly as written — this is the "override".

**Substitute before building the box**, so the box's width/wrap/border math uses the
real (post-substitution) text and stays aligned. The content-assembly functions
(`build_about_boxes`, `now_boxes`, `projects_boxes`) take `domain`/`email` params,
`.replace()` the tokens in the raw strings, then build boxes as today. Nav hrefs in the
nav box are prefixed with `base_url`.

### 4. Per-post OG image

`MarkdownPost` gains `social: Option<String>` (frontmatter `social:`). Resolution:

- `social:` present → `og_image = {base_url}/post/assets/{social}`
  (e.g. `social: namecheap/social.png` → `{base_url}/post/assets/namecheap/social.png`).
- absent → default `og_image = {base_url}/static/og-image.gif`.
- Non-post pages → default `{base_url}/static/og-image.gif`.

This also fixes the current bug where non-post pages reference the nonexistent
`og-image.png`. (The namecheap post will set `social: namecheap/social.png`; the user
adds that file to `posts/assets/namecheap/social.png`.)

### 5. Post image paths → root-absolute

In `markdown.rs`, rewrite post-body image sources `src="assets/…"` → `src="/post/assets/…"`
so images are root-absolute. Then they load correctly whether the post is served at
`/post/namecheap-wtf` (zoa.sh/vx.gay) or at the root `/` (namecheap.wtf). Narrow,
targeted replace — only the `assets/` prefix used by post images is affected.

### 6. RSS / sitemap

`rss_feed` and `sitemap` currently hardcode `https://zoa.sh`. Use the effective
`base_url` for the requesting host (so `vx.gay/rss.xml` yields vx.gay links, etc.).

## Caddy changes (`femboy/deploy` → `servers/home/incus/caddy/Caddyfile`)

Replace the current public-web blocks:

### curl-aware :80

On `:80`, serve `curl`/`wget` over plain HTTP; upgrade everyone else to HTTPS.

```caddy
http://vx.gay, http://zoa.sh {
	@cli header_regexp User-Agent (?i)(curl|wget)
	handle @cli { reverse_proxy zoa.incus:8084 }
	handle { redir https://{host}{uri} }        # 302; Vary: User-Agent (UA-conditional)
}
http://namecheap.wtf {
	@cli header_regexp User-Agent (?i)(curl|wget)
	handle @cli {
		rewrite / /post/namecheap-wtf
		reverse_proxy zoa.incus:8084
	}
	handle { redir https://{host}{uri} }
}
```

Redirect is **302** (not 301) because the response varies by User-Agent; a permanent
cache entry could be wrong for the other UA class. Add `header Vary User-Agent` on the
redirect path.

### namecheap.wtf serve-in-place (:443)

Rewrite **only the root** `/` to the post; everything else (`/static`, `/post/assets`,
`/cat`, …) passes through untouched, so CSS/JS/wasm/images load normally.

```caddy
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

## Files touched

**zoa.sh**
- `src/mods/site.rs` (new) — effective-domain resolver, registered in `mods.rs`.
- `src/main.rs` — resolve `Site` per handler; pass to templates + content builders;
  RSS/sitemap use `base_url`.
- `templates/*.tera` (index, about, projects, uses, now, blog, post) — OG/canonical/nav
  via `base_url`/`og_image`.
- `src/mods/about.rs`, `now.rs`, `projects.rs`, `data.rs` — `{domain}`/`{email}` tokens,
  absolute nav hrefs.
- `src/mods/markdown.rs` — `social:` frontmatter + `assets/` → `/post/assets/` rewrite.
- `posts/namecheap.md` — add `social: namecheap/social.png`.

**femboy/deploy**
- `servers/home/incus/caddy/Caddyfile` — curl-aware `:80` + namecheap root-only rewrite,
  then redeploy Caddy (reload).

## Verification

1. `Host: zoa.sh` → OG/canonical/nav/email all `zoa.sh` / `zoa@zoa.sh`.
2. `Host: vx.gay` → all `vx.gay` / `z@vx.gay`; `curl {domain}` reads `curl vx.gay`.
3. `Host: namecheap.wtf` on `/` → the namecheap post renders; nav → `vx.gay/<page>`;
   OG image = the post's `social.png`; CSS/JS/images load.
4. Per-post OG: namecheap post → `social.png`; other posts/pages → `og-image.gif`.
5. Repo link `github.com/vxfemboy/zoa.sh` unchanged on every domain (override works).
6. Caddy: `curl -sI http://zoa.sh` → 200 (served); browser UA → 302 → https. `curl zoa.sh`
   still renders the ANSI site. namecheap.wtf CSS 200 (not the post HTML).
7. `cargo fmt`/`clippy`/`test` clean; deploy via `./deploy.sh`; Caddy reload.

## Out of scope / notes

- No app-level redirects or `/p/{id}` short links (previously removed — namecheap.wtf is
  handled entirely at Caddy + effective-domain rendering).
- Assets stay root-relative; **no** changes to `cat.js` / `scroll-container.js`.
- Author identity stays `vxfemboy` on all domains; only domain/email/URLs swap.
