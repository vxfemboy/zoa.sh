// This file is only compiled for WASM target
#[cfg(target_arch = "wasm32")]
mod mods {
    pub mod wasm;
}

// Re-export for WASM target
#[cfg(target_arch = "wasm32")]
pub use mods::wasm::*;
