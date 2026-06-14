//! Generate the /about profile portrait as truecolor ANSI half-blocks, from source.
//!
//! Fetches the GitHub avatar (or a local/remote image given as an arg), converts
//! it with the `image` crate using the half-block technique — each character cell
//! is `▀` (upper half block) whose foreground is the top pixel and background is
//! the bottom pixel, so one cell encodes two vertical pixels — and frames it in
//! the same border the sidebar boxes use.
//!
//! Writes two committed assets the server serves:
//!   templates/ascii/avatar.html  — colored <span> grid for the browser
//!   templates/ascii/avatar.ans   — raw truecolor ANSI for the curl/terminal view
//!
//! Run:  cargo run --bin gen-avatar [URL_OR_PATH]
//! Default source: https://github.com/vxfemboy.png

use image::imageops::FilterType;
use std::error::Error;

// Match the sidebar boxes (CATEGORIES_WIDTH_LARGE = 55): 2 border cols + a
// 1-space margin each side leaves CELLS columns of portrait.
const BOX_WIDTH: usize = 55;
const CONTENT_WIDTH: usize = BOX_WIDTH - 2; // inside the ║ ║ borders
const CELLS: usize = CONTENT_WIDTH - 2; // minus the 1-space margins
const TITLE: &str = "PFP";
const HALF: char = '\u{2580}'; // ▀ upper half block

fn main() -> Result<(), Box<dyn Error>> {
    let src = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://github.com/vxfemboy.png".to_string());

    let bytes = if src.starts_with("http://") || src.starts_with("https://") {
        eprintln!("fetching {src} ...");
        reqwest::blocking::get(&src)?
            .error_for_status()?
            .bytes()?
            .to_vec()
    } else {
        eprintln!("reading {src} ...");
        std::fs::read(&src)?
    };

    let img = image::load_from_memory(&bytes)?;
    let (iw, ih) = (img.width() as usize, img.height() as usize);

    // Square-ish per cell: each cell is 2 px tall, so a `CELLS`-wide portrait of
    // an iw×ih image needs `rows` character rows where 2*rows px tall preserves
    // aspect.
    let px_w = CELLS as u32;
    let rows = ((CELLS * ih) as f64 / iw as f64 / 2.0).round().max(1.0) as usize;
    let px_h = (rows * 2) as u32;

    let rgb = img.resize_exact(px_w, px_h, FilterType::Lanczos3).to_rgb8();

    // Collect (top, bottom) rgb per cell row.
    let mut cell_rows: Vec<Vec<([u8; 3], [u8; 3])>> = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut row = Vec::with_capacity(CELLS);
        for x in 0..CELLS as u32 {
            let top = rgb.get_pixel(x, (r * 2) as u32).0;
            let bottom = rgb.get_pixel(x, (r * 2 + 1) as u32).0;
            row.push((top, bottom));
        }
        cell_rows.push(row);
    }

    std::fs::write("templates/ascii/avatar.html", render_html(&cell_rows))?;
    std::fs::write("templates/ascii/avatar.ans", render_ansi(&cell_rows))?;
    eprintln!(
        "wrote templates/ascii/avatar.html and avatar.ans ({CELLS}x{rows} cells, {BOX_WIDTH}-col box)"
    );
    Ok(())
}

fn h_line(left: char, fill: char, right: char) -> String {
    format!("{left}{}{right}", fill.to_string().repeat(CONTENT_WIDTH))
}

fn title_line() -> String {
    let pad = CONTENT_WIDTH - TITLE.len();
    let l = pad / 2;
    let r = pad - l;
    format!("║{}{TITLE}{}║", " ".repeat(l), " ".repeat(r))
}

/// Colored `<span>` grid for the browser (placed inside <pre class="profile-art">).
fn render_html(cells: &[Vec<([u8; 3], [u8; 3])>]) -> String {
    let mut out = String::new();
    out.push_str(&h_line('╔', '═', '╗'));
    out.push('\n');
    out.push_str(&title_line());
    out.push('\n');
    out.push_str(&h_line('╠', '═', '╣'));
    out.push('\n');
    for row in cells {
        out.push_str("║ ");
        for (top, bottom) in row {
            out.push_str(&format!(
                "<span style=\"color:rgb({},{},{});background:rgb({},{},{})\">{HALF}</span>",
                top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]
            ));
        }
        out.push_str(" ║\n");
    }
    out.push_str(&h_line('╚', '═', '╝'));
    out.push('\n');
    out
}

/// Raw truecolor ANSI for the curl/terminal view.
fn render_ansi(cells: &[Vec<([u8; 3], [u8; 3])>]) -> String {
    const RESET: &str = "\x1b[0m";
    let mut out = String::new();
    out.push_str(&h_line('╔', '═', '╗'));
    out.push('\n');
    out.push_str(&title_line());
    out.push('\n');
    out.push_str(&h_line('╠', '═', '╣'));
    out.push('\n');
    for row in cells {
        out.push_str("║ ");
        for (top, bottom) in row {
            out.push_str(&format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{HALF}",
                top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]
            ));
        }
        out.push_str(RESET);
        out.push_str(" ║\n");
    }
    out.push_str(&h_line('╚', '═', '╝'));
    out.push('\n');
    out
}
