#!/usr/bin/env bash
# Development auto-reload: rebuild + restart the server on every save.
#
# WASM is built automatically by build.rs as part of `cargo run`, so the watcher
# only has to re-run cargo — one command covers both the server and the cat.
#
# Picks whatever file-watcher you have installed. Install one of:
#   watchexec   (recommended)  cargo install watchexec-cli   |  brew install watchexec
#   cargo-watch                cargo install cargo-watch
set -euo pipefail

cd "$(dirname "$0")"

# Watched extensions: Rust, templates, css, js, config, and markdown posts.
EXTS=(rs tera html css js toml md)
RUN_CMD="cargo run"

if command -v watchexec >/dev/null 2>&1; then
    # Ignore the generated portrait: the server rewrites it on startup (pfp
    # auto-update), and watching it would cause a restart loop.
    exec watchexec --restart --watch src --watch templates --watch static --watch config.toml --watch posts \
        --exts "$(IFS=,; echo "${EXTS[*]}")" \
        --ignore 'static/wasm/**' --ignore 'target/**' --ignore 'templates/ascii/avatar.*' \
        -- $RUN_CMD
elif command -v cargo-watch >/dev/null 2>&1; then
    exec cargo watch \
        -w src -w templates -w static -w config.toml -w posts \
        --ignore 'static/wasm/*' --ignore 'templates/ascii/avatar.*' \
        -x run
else
    echo "No file watcher found. Install one of:"
    echo "  cargo install watchexec-cli     # recommended"
    echo "  cargo install cargo-watch"
    echo
    echo "Then re-run ./dev.sh   (plain 'cargo run' already builds the WASM via build.rs)."
    exit 1
fi
