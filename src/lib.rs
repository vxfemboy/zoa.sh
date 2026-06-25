// WASM target: the interactive cat.
#[cfg(target_arch = "wasm32")]
mod mods {
    pub mod wasm;
}
#[cfg(target_arch = "wasm32")]
pub use mods::wasm::*;

// Native target: shared image → ANSI half-block converter, used by the server
// (startup pfp auto-refresh) and the `gen-avatar` binary.
#[cfg(not(target_arch = "wasm32"))]
pub mod ansi_image;
