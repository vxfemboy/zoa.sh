//! Build script: compile the WASM module as part of a normal `cargo build`/`cargo run`
//! so the interactive cat is always in sync without a separate `./build-wasm.sh` step.
//!
//! Guards:
//! - Skips when building for the `wasm32` target (this is the inner build wasm-pack
//!   itself triggers — running it again would recurse forever).
//! - Skips when `SKIP_WASM=1` (CI release builds set this and run build-wasm.sh
//!   explicitly; also useful for anyone without wasm-pack installed).
//! - Runs the inner wasm build in its own target dir (`target/wasm-build`) so it
//!   doesn't deadlock on the outer build's `target/` lock.
//! - Fails soft: a missing/broken wasm-pack warns but does not fail the build.

use std::path::Path;
use std::process::Command;

fn main() {
    // Only rebuild wasm when the things it's built from change.
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/mods/wasm.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=SKIP_WASM");

    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("wasm32") {
        // We're the inner build invoked by wasm-pack; do not recurse.
        return;
    }
    if std::env::var("SKIP_WASM").is_ok() {
        println!("cargo:warning=SKIP_WASM set — skipping WASM build");
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let wasm_target_dir = Path::new(&manifest_dir).join("target/wasm-build");

    let status = Command::new("wasm-pack")
        .current_dir(&manifest_dir)
        // Isolate the inner build's target dir to avoid a lock deadlock with the
        // outer cargo invocation that is running this build script.
        .env("CARGO_TARGET_DIR", &wasm_target_dir)
        .args([
            "build",
            "--target",
            "web",
            "--out-dir",
            "static/wasm",
            "--no-typescript",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => println!("cargo:warning=wasm-pack exited with {s}; static/wasm may be stale"),
        Err(e) => println!(
            "cargo:warning=wasm-pack not run ({e}); install it or run ./build-wasm.sh manually"
        ),
    }
}
