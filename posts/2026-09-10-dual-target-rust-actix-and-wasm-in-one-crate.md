---
title: Taming the Dual-Target Rust Crate: Native Actix-Web + Browser WASM in One Cargo Workspace
short_title: Dual-Target Rust Crate
subtitle: Native Actix-Web + Browser WASM in One Cargo Workspace
date: 2026-09-10
slug: dual-target-rust-actix-and-wasm-in-one-crate
tags: rust, wasm, web, actix, architecture, sysadmin
---

# Taming the Dual-Target Rust Crate: Native Actix-Web + Browser WASM in One Cargo Workspace

## Table of Contents
1. [The Vision: Zero JavaScript Framework Bloat](#the-vision-zero-javascript-framework-bloat)
2. [The Dual-Target Architecture](#the-dual-target-architecture)
3. [Cargo.toml Conditional Dependencies](#cargotoml-conditional-dependencies)
4. [The build.rs Concurrency Deadlock](#the-buildrs-concurrency-deadlock)
5. [Connecting the Browser Cat to WASM](#connecting-the-browser-cat-to-wasm)
6. [Testing and CI Verification](#testing-and-ci-verification)
7. [References](#references)

---

## The Vision: Zero JavaScript Framework Bloat

If you open [zoa.sh](https://zoa.sh), you notice immediately that it's unlike typical modern websites:
- It serves server-rendered HTML wrapped in responsive, pixel-perfect ASCII-art boxes.
- If you curl it from a terminal (`curl https://zoa.sh`), an Actix-web middleware intercepts the request and returns full ANSI terminal art.
- If you open it in a desktop browser, an interactive WASM cat follows your cursor around the screen.
- **There is zero npm, zero webpack, zero React, and zero 50MB node_modules directory.**

Everything—the native HTTP web server, the template engine, the syntax highlighter, and the browser WASM module—is compiled from **a single Rust codebase** in [`zoa-sh`](https://github.com/vxfemboy/zoa.sh).

Compiling a single Rust crate to two completely different target architectures (`x86_64-unknown-linux-gnu` for the server and `wasm32-unknown-unknown` for the browser) in a single build command is an exercise in Cargo wizardry.

---

## The Dual-Target Architecture

```text
                               +----------------------------+
                               |     zoa-sh Cargo Crate     |
                               +----------------------------+
                                      /              \
         cfg(not(target_arch = "wasm32"))             cfg(target_arch = "wasm32")
                                    /                  \
                                   v                    v
                       +----------------------+    +------------------------+
                       |    src/main.rs       |    |       src/lib.rs       |
                       | - Actix-Web Server   |    | - wasm-bindgen         |
                       | - Tera Templates     |    | - Browser Cat Canvas   |
                       | - Syntect Highlight  |    | - Event Loop           |
                       +----------------------+    +------------------------+
                                   |                            |
                                   v                            v
                         [Native Server Binary]      [static/wasm/zoa_sh.wasm]
```

To make this work:
1. `src/main.rs` is the server entrypoint.
2. `src/lib.rs` is the WebAssembly browser module.
3. In `Cargo.toml`, we declare `[lib] crate-type = ["cdylib", "rlib"]` so the compiler can emit both a static library for the server and a dynamic WebAssembly bundle for the client.

---

## Cargo.toml Conditional Dependencies

The server needs heavy dependencies: `actix-web`, `tera`, `syntect`, `scraper`, and `image`. None of these compile to `wasm32` without pulling in complex C shims or failing on thread primitives.

Conversely, the browser module needs `wasm-bindgen`, `web-sys`, and `js-sys`, which are meaningless to an Actix server binary.

In `Cargo.toml`, we gate dependencies using target filters:

```toml
[package]
name = "zoa-sh"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

# Native server dependencies
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
actix-web = "4"
tera = "1"
syntect = "5"
pulldown-cmark = "0.10"
scraper = "0.19"
image = "0.25"

# Browser WebAssembly dependencies
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["Window", "Document", "Element", "Performance"] }
js-sys = "0.3"
```

---

## The build.rs Concurrency Deadlock

We wanted the developer experience to be seamless: running `cargo run` should automatically compile the latest browser WASM code into `static/wasm/` before starting the HTTP server.

Our first attempt in `build.rs` called `wasm-pack build --target web --out-dir static/wasm`.

It resulted in an immediate **build deadlock**:
- `cargo run` acquired the file lock on `target/debug/.cargo-lock`.
- `build.rs` launched `wasm-pack build`.
- `wasm-pack` invoked `cargo build --target wasm32-unknown-unknown`.
- The child `cargo` process attempted to acquire the same `target/debug/.cargo-lock`, blocked indefinitely, and hung the compiler!

### The Solution
We configured `build.rs` to redirect the WASM compilation into a dedicated, isolated target directory (`target/wasm-build`), and skipped WASM builds when cross-compiling or when `SKIP_WASM=1` is set:

```rust
// build.rs snippet
use std::process::Command;
use std::env;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let skip_wasm = env::var("SKIP_WASM").is_ok();

    if target.contains("wasm32") || skip_wasm {
        return;
    }

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/mods/wasm.rs");

    let status = Command::new("wasm-pack")
        .args([
            "build",
            "--target", "web",
            "--out-dir", "static/wasm",
            "--",
            "--target-dir", "target/wasm-build"
        ])
        .status()
        .expect("Failed to execute wasm-pack");

    assert!(status.success(), "WASM build failed");
}
```

---

## Connecting the Browser Cat to WASM

Inside `src/mods/wasm.rs`, the cat animation state machine runs at 60 FPS:

```rust
#[wasm_bindgen]
pub struct CatEngine {
    x: f64,
    y: f64,
    target_x: f64,
    target_y: f64,
    state: CatState,
}

#[wasm_bindgen]
impl CatEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0, target_x: 0.0, target_y: 0.0, state: CatState::Idle }
    }

    pub fn tick(&mut self, cursor_x: f64, cursor_y: f64) -> String {
        self.target_x = cursor_x;
        self.target_y = cursor_y;
        self.update_physics();
        self.render_ascii_frame()
    }
}
```

The browser loads `static/wasm/zoa_sh.js`, hooks `window.onmousemove`, and renders the ASCII frames into a fixed overlay container.

---

## Testing and CI Verification

In CI, unit tests and server handlers are verified independently:
```bash
SKIP_WASM=1 cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
Running `cargo test` executes the 23 native unit tests without having to install `wasm-pack` on lightweight CI runners.

Dual-target compilation keeps your stack unified, blazingly fast, and completely free of JavaScript runtime bloat.

---

## References
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [Actix-Web Framework](https://actix.rs/)
- [Cargo Target-Specific Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#target-specific-dependencies)
