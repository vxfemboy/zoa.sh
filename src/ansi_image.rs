//! Shared image → truecolor ANSI half-block converter (native only).
//!
//! Used by both the server (auto-refresh on startup, driven by `[pfp]` config)
//! and the `gen-avatar` helper binary. Each character cell is `▀` (upper half
//! block) whose foreground is the top pixel and background is the bottom pixel,
//! so one cell encodes two vertical pixels. The grid is framed in the same
//! border the sidebar boxes use.

use image::imageops::FilterType;
use std::error::Error;
use std::time::Duration;

const HALF: char = '\u{2580}'; // ▀ upper half block
const TITLE: &str = "PFP";

/// Tunable conversion options (sourced from `[pfp]` in config.toml).
pub struct PfpOptions {
    /// Portrait width in character cells (box width = `width_cells + 4`).
    pub width_cells: usize,
    /// Added to each channel after contrast, in [-1.0, 1.0]. 0.0 = none.
    pub brightness: f32,
    /// Multiplier around mid-gray. 1.0 = none, >1 punchier, <1 flatter.
    pub contrast: f32,
}

impl Default for PfpOptions {
    fn default() -> Self {
        Self {
            width_cells: 51,
            brightness: 0.0,
            contrast: 1.0,
        }
    }
}

fn adjust(c: u8, brightness: f32, contrast: f32) -> u8 {
    let v = c as f32 / 255.0;
    let v = (v - 0.5) * contrast + 0.5 + brightness;
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Fetch image bytes from an http(s) URL (8s timeout, follows redirects) or a
/// local file path.
pub fn fetch_bytes(src: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    if src.starts_with("http://") || src.starts_with("https://") {
        let resp = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()?
            .get(src)
            .send()?
            .error_for_status()?;
        Ok(resp.bytes()?.to_vec())
    } else {
        Ok(std::fs::read(src)?)
    }
}

/// Decode image bytes and render the framed portrait as `(html, ansi)`.
pub fn image_to_assets(
    bytes: &[u8],
    opts: &PfpOptions,
) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    let cells = opts.width_cells.max(8);
    let content_width = cells + 2; // 1-space margin each side, inside the ║ borders

    let img = image::load_from_memory(bytes)?;
    let (iw, ih) = (img.width() as usize, img.height() as usize);
    let px_w = cells as u32;
    let rows = (((cells * ih) as f64) / (iw as f64) / 2.0).round().max(1.0) as usize;
    let rgb = img
        .resize_exact(px_w, (rows * 2) as u32, FilterType::Lanczos3)
        .to_rgb8();

    let mut cell_rows: Vec<Vec<([u8; 3], [u8; 3])>> = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut row = Vec::with_capacity(cells);
        for x in 0..cells as u32 {
            let t = rgb.get_pixel(x, (r * 2) as u32).0;
            let b = rgb.get_pixel(x, (r * 2 + 1) as u32).0;
            let adj = |p: [u8; 3]| {
                [
                    adjust(p[0], opts.brightness, opts.contrast),
                    adjust(p[1], opts.brightness, opts.contrast),
                    adjust(p[2], opts.brightness, opts.contrast),
                ]
            };
            row.push((adj(t), adj(b)));
        }
        cell_rows.push(row);
    }

    Ok((
        render(&cell_rows, content_width, Mode::Html),
        render(&cell_rows, content_width, Mode::Ansi),
    ))
}

/// Fetch from `url`, convert, and write both committed assets.
pub fn refresh(
    url: &str,
    opts: &PfpOptions,
    html_path: &str,
    ans_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bytes = fetch_bytes(url)?;
    let (html, ans) = image_to_assets(&bytes, opts)?;
    std::fs::write(html_path, html)?;
    std::fs::write(ans_path, ans)?;
    Ok(())
}

enum Mode {
    Html,
    Ansi,
}

fn h_line(left: char, fill: char, right: char, content_width: usize) -> String {
    format!("{left}{}{right}", fill.to_string().repeat(content_width))
}

fn title_line(content_width: usize) -> String {
    let pad = content_width.saturating_sub(TITLE.len());
    let l = pad / 2;
    format!("║{}{TITLE}{}║", " ".repeat(l), " ".repeat(pad - l))
}

fn render(cells: &[Vec<([u8; 3], [u8; 3])>], content_width: usize, mode: Mode) -> String {
    const RESET: &str = "\x1b[0m";
    let mut out = String::new();
    out.push_str(&h_line('╔', '═', '╗', content_width));
    out.push('\n');
    out.push_str(&title_line(content_width));
    out.push('\n');
    out.push_str(&h_line('╠', '═', '╣', content_width));
    out.push('\n');
    for row in cells {
        out.push_str("║ ");
        for (t, b) in row {
            match mode {
                Mode::Html => out.push_str(&format!(
                    "<span style=\"color:rgb({},{},{});background:rgb({},{},{})\">{HALF}</span>",
                    t[0], t[1], t[2], b[0], b[1], b[2]
                )),
                Mode::Ansi => out.push_str(&format!(
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{HALF}",
                    t[0], t[1], t[2], b[0], b[1], b[2]
                )),
            }
        }
        if let Mode::Ansi = mode {
            out.push_str(RESET);
        }
        out.push_str(" ║\n");
    }
    out.push_str(&h_line('╚', '═', '╝', content_width));
    out.push('\n');
    out
}
