//! Manually regenerate the /about profile portrait (ANSI half-block) from an
//! image, using the shared `ansi_image` converter.
//!
//! Run:  cargo run --bin gen-avatar [URL_OR_PATH]
//! Default source: https://github.com/vxfemboy.png
//!
//! The running server can also do this automatically on startup — see the
//! `[pfp]` section in config.toml.

use std::error::Error;
use zoa_sh::ansi_image::{self, PfpOptions};

const HTML: &str = "templates/ascii/avatar.html";
const ANS: &str = "templates/ascii/avatar.ans";

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let src = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://github.com/vxfemboy.png".to_string());

    eprintln!("converting {src} ...");
    ansi_image::refresh(&src, &PfpOptions::default(), HTML, ANS)?;
    eprintln!("wrote {HTML} and {ANS}");
    Ok(())
}
