---
title: Building This Site with Rust and WASM
date: 2024-12-28
slug: building-this-site
tags: rust, webdev, wasm
---

This entire site is powered by Rust - both the backend server and the client-side interactivity via WebAssembly.

## The Stack

```rust
// Server-side with Actix-web
async fn index(tera: web::Data<Tera>) -> Result<HttpResponse> {
    let content = ContentManager::new();
    let ctx = content.create_page_context()?;
    let rendered = tera.render("index.html.tera", &ctx)?;
    Ok(HttpResponse::Ok().body(rendered))
}
```

## Features

- **Responsive ASCII boxes** - Adapts to any screen size
- **Real-time shoutbox** - WebSocket-powered chat
- **Markdown blog** - Write posts in markdown, rendered with syntax highlighting
- **Dynamic stars** - Twinkling background generated server-side

## Why Rust?

Performance, safety, and because I like suffering (in a good way). The entire site compiles to a single binary with no runtime dependencies.
