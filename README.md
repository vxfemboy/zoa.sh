```
                     ╔═══════════════════════════════════════════════════════════════╗
                     ║                                                               ║
                     ║      ███████╗ █████╗ ██████╗ ███████╗██╗████████╗███████╗     ║
                     ║      ██╔════╝██╔══██╗██╔══██╗██╔════╝██║╚══██╔══╝██╔════╝     ║
                     ║      ███████╗███████║██║  ██║███████╗██║   ██║   █████╗       ║
                     ║      ╚════██║██╔══██║██║  ██║╚════██║██║   ██║   ██╔══╝       ║
                     ║      ███████║██║  ██║██████╔╝███████║██║   ██║   ███████╗     ║
                     ║      ╚══════╝╚═╝  ╚═╝╚═════╝ ╚══════╝╚═╝   ╚═╝   ╚══════╝     ║
                     ║                                                               ║
                     ║             ascii art website engine in rust                  ║
                     ║                                                               ║
                     ╚═══════════════════════════════════════════════════════════════╝
```

<div align="center">

**[zoa.sh](https://zoa.sh)** | retro web aesthetics for the modern age

`rust` `actix-web` `wasm` `ascii-art` `markdown` `syntax-highlighting`

</div>

---

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  WHAT IS THIS                                                                │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  a rust-powered website that generates responsive ASCII art boxes,           │
│  renders markdown blog posts with syntax highlighting, and features          │
│  an interactive WASM cat that follows your cursor around                     │
│                                                                              │
│  built for https://zoa.sh - personal site of a gay femboy hacker          │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

## features

```
  ╔════════════════════════════════════════════════════════════════════════╗
  ║                                                                        ║
  ║   [x] dynamic ASCII box generation with responsive breakpoints         ║
  ║   [x] markdown blog with YAML frontmatter + syntax highlighting        ║
  ║   [x] real-time shoutbox via websockets                                ║
  ║   [x] interactive WASM cat (follows mouse, idle animations)            ║
  ║   [x] proper unicode/emoji handling with twemoji fallbacks             ║
  ║   [x] tag-based post filtering                                         ║
  ║   [x] RSS feed generation                                              ║
  ║   [x] SEO (sitemap, robots.txt, meta tags)                             ║
  ║                                                                        ║
  ╚════════════════════════════════════════════════════════════════════════╝
```

## quickstart

```bash
# clone it
git clone https://github.com/vxfemboy/sadsite
cd sadsite

# build the wasm cat
./build-wasm.sh

# run it
cargo run

# visit http://localhost:8080
```

## structure

```
src/
├── main.rs              # actix-web server, routes, handlers
├── lib.rs               # wasm exports
└── mods/
    ├── ascii_art.rs     # box generation, unicode width calc
    ├── markdown.rs      # blog post parser + syntax highlighting
    ├── shoutbox.rs      # websocket chat system
    ├── wasm.rs          # interactive cat logic
    ├── responsive.rs    # multi-breakpoint box builder
    ├── content.rs       # page context assembly
    ├── data.rs          # site content/nav items
    ├── constants.rs     # widths, breakpoints, config
    └── ...

posts/                   # markdown blog posts go here
templates/               # tera html templates
static/                  # css, js, fonts, images
```

## blog posts

drop `.md` files in `posts/` with frontmatter:

```markdown
---
title: your post title
date: 2024-12-28
tags: rust, hacking, uwu
---

# your content here

code blocks get syntax highlighting automatically
```

## api

```
GET  /                      main page
GET  /blog                  blog index (supports ?tag=filter)
GET  /post/{slug}           individual post
GET  /rss.xml               rss feed
GET  /sitemap.xml           sitemap
GET  /api/posts             json post list
GET  /api/health            health check
WS   /ws/shoutbox           shoutbox websocket
```

## config

create `config.toml`:

```toml
debug = false
cache_enabled = true
cache_capacity = 1000

[server]
host = "127.0.0.1"
port = 8080
```

---

<div align="center">

```
        /\_/\
       ( o.o )
        > ^ <
       /|   |\
      (_|   |_)
```

made with mass amounts of mass

**[vxfemboy](https://github.com/vxfemboy)**

</div>
