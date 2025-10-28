// This file is only compiled for WASM target
#[cfg(target_arch = "wasm32")]
mod mods {
    pub mod wasm;
    pub mod shoutbox_protocol;
}

// Re-export for WASM target
#[cfg(target_arch = "wasm32")]
pub use mods::{shoutbox_protocol::*, wasm::*};
