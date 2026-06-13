---
title: Rust to WebAssembly - A Practical Guide
date: 2024-12-10
slug: rust-wasm
tags: rust, wasm, tutorial
---

WebAssembly lets you run Rust in the browser. Here's how this site uses it.

## Setup

```bash
# Install wasm-pack
cargo install wasm-pack

# Build the WASM module
wasm-pack build --target web --out-dir static/wasm
```

## Cargo.toml Config

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["Document", "Element"] }
```

## Exposing Functions to JavaScript

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn render_ascii_cat(state: &str) -> String {
    match state {
        "sleep" => include_str!("../templates/ascii/cat/sleep1.txt"),
        "run" => include_str!("../templates/ascii/cat/run1.txt"),
        _ => include_str!("../templates/ascii/cat/still.txt"),
    }.to_string()
}
```

## Loading in HTML

```javascript
import init, { render_ascii_cat } from '/static/wasm/zoa_sh.js';

await init();
const cat = render_ascii_cat("sleep");
document.getElementById("cat").textContent = cat;
```

## Why WASM?

- Near-native performance
- Share code between server and client
- Type safety all the way down
