// Export modules for both WASM and non-WASM targets
pub mod mods;

// Re-export modules
pub use mods::*;

// WASM-specific exports
#[cfg(target_arch = "wasm32")]
pub use mods::wasm::*;
