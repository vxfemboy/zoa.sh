```
                                                                      _
       ..       .:, .                                             `*-.
                 ..,'        ...  'cdOOl.  .:dko,,ko.              )  _`-.
            .':ox0K0l.      'kOccoo:;kW0:;dxo;.  ;XNc             .  : `. .
       .,coxkkdxKKl.       .dN0d:.  .xWXOo;.     ,KW0,            : _   '  \
 .codxkkxdc,..;xx'        ,xXK:   .:xXNd.        ;KNNx.           ; *` _.   `*-._
 .:ol:'.    'dk:        ;doxNk..:dxdkNK;         :0dkNl           `-.-'          `-.
          .lOl.   ..  'xx,.oN0xkd:..xWd.         l0;;KK;    ...      ;       `       `.
        .:kd'    'kd..ONd:dKNk;.   :X0,          o0,.xN0dooool;      :.       .        \
       ,xx;      .k0,.oOOxxXX:    'OXc      ..';l0XkxxONKl..         . \  .   :   .-'   .
     .dk:.        ,OO; .  ,0K,   .dNo.     xOOkxOXd'. .kNd.          '  `+.;  ;  '      :
   .l0Olcodxxxkkkkx0NXd'  ,KK,  .oXd.      ...  lO,    'ONd.         :  '  |    ;       ;-.
  :ONXOxdl:;,'''',:cokKO  ;KK, .dKl.           .Ox.     'ONx.        ; '   : :`-:     _.`* ;
.xKxc,.               ..  ,KNockk,             lK:       .kN0:     .*' /  .*' ; .*`- +'  `*'
.:'                       .dXKkc.             '0k.        .lx;     `*-*   `*-*  `*-*'
                            ..                .;.
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
│  built for https://zoa.sh - personal site of a gay femboy                    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

## features

```
  ╔════════════════════════════════════════════════════════════════════════╗
  ║                                                                        ║
  ║   [x] dynamic ASCII box generation with responsive breakpoints         ║
  ║   [x] markdown blog with YAML frontmatter + syntax highlighting        ║
  ║   [x] curl/ansi terminal rendering (curl zoa.sh)                       ║
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
git clone https://github.com/vxfemboy/zoa.sh
cd zoa.sh

# run it — `cargo run` builds the wasm cat automatically (via build.rs)
cargo run

# visit http://localhost:8080
# or, from a terminal:  curl http://localhost:8080
```

## development

```bash
# auto-reload: rebuilds wasm + restarts the server on save
./dev.sh
# (install a watcher first: `cargo install watchexec-cli` or `cargo install cargo-watch`)

# plain run — build.rs compiles the wasm into static/wasm as part of the build
cargo run

# skip the implicit wasm build (faster server-only rebuilds; CI uses this)
SKIP_WASM=1 cargo run

# build the wasm by hand (what CI and SKIP_WASM=1 builds rely on)
./build-wasm.sh
```

`curl`/`wget` clients get an ANSI-rendered terminal version of any page
(the HTML is converted to colored plaintext using the site's own CSS). See the
[actix auto-reload docs](https://actix.rs/docs/autoreload/) for background.

The `/about` portrait is the GitHub avatar rendered as truecolor ANSI half-blocks
(`▀` cells, two pixels each), converted from source by the `gen-avatar` binary and
committed to `templates/ascii/avatar.html` (browser) + `avatar.ans` (curl/terminal).
Regenerate it with:

```bash
cargo run --bin gen-avatar                 # fetches github.com/vxfemboy.png
cargo run --bin gen-avatar -- ~/dl/me.png  # or convert a local image
```

## structure

```
src/
├── main.rs              # actix-web server, routes, handlers
├── lib.rs               # wasm exports
└── mods/
    ├── ascii_art.rs     # box generation, unicode width calc
    ├── markdown.rs      # blog post parser + syntax highlighting
    ├── text.rs          # curl/ANSI terminal rendering (all pages)
    ├── about.rs         # /about page content + sections
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

# any page returns ANSI plaintext when requested with a curl/wget User-Agent
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
