//! Content for the `/about` page — Zoa's bio, rendered into responsive ASCII
//! boxes. Authored from the LinkedIn profile, kept in the site's playful voice.
//!
//! Each section is built at all four breakpoints via `create_header_box`, with
//! prose pre-wrapped to the matching `WRAP_WIDTH_*` (the box renderer splits on
//! `\n` but does not wrap on its own). Bullet/link lines are left intact.

use crate::mods::constants::*;
use crate::mods::{create_header_box, wrap_text, BoxSizes};
use serde::Serialize;
use unicode_width::UnicodeWidthStr;

pub struct AboutBoxes {
    pub summary_box: BoxSizes,
    pub skills_box: BoxSizes,
    pub education_box: BoxSizes,
    pub links_box: BoxSizes,
}

/// A single role for the interactive experience timeline (rendered as a `tree`-
/// style chart with click-to-expand bios).
#[derive(Serialize)]
pub struct Job {
    /// Padded to a fixed width so the tree columns align in monospace / curl.
    pub date: String,
    pub company: String,
    pub role: String,
    pub bio: String,
}

/// Curated work history, newest first. Full voice retained.
pub fn experience() -> Vec<Job> {
    let rows: &[(&str, &str, &str, &str)] = &[
        (
            "2024–now",
            "femboy cyber networks",
            "founder",
            "building an ISP that actually gives a damn about the people incumbents forgot. BGP peering, fiber, upstream wrangling. yes the name is real. yes the ASN is live.",
        ),
        (
            "2026",
            "dash crystal",
            "research software engineer",
            "applied AI research: LLM fine-tuning (LoRA + multi-GPU full fine-tunes), eval harnesses, training-observability infra, systems instrumentation.",
        ),
        (
            "2023–24",
            "occamsec",
            "software engineer",
            "frontend + backend for InCenter, an automated breach-and-attack simulation platform. shipped the infra on AWS with terraform + ansible.",
        ),
        (
            "2022–23",
            "iproyal",
            "network software engineer",
            "network software engineering on proxy infrastructure at scale. yes i thought about packets constantly. yes that was fine.",
        ),
        (
            "2020–23",
            "stealth AI startup",
            "founder & engineer",
            "solo-built an AI automation + marketing company. reverse-engineered platform APIs, ran stable diffusion pipelines before \"generative AI\" was a buzzword, conversational agents, hands-free multi-platform automation, automated payouts. mostly adult content creators -- pays better, problems are more interesting. scaled it solo until someone bought the whole thing. \"before it was cool.\"",
        ),
        (
            "2021–22",
            "filmtek cloud",
            "embedded linux engineer",
            "custom kernels, ported linux to embedded devices, kernel + socket level code, firewall/security software, hand-applied firmware patches. ran daily pentests, red/blue teams.",
        ),
        (
            "2021",
            "goldman sachs",
            "unix sysadmin / IT engineer",
            "domain user/group management, rewrote linux docs that hadn't been touched in years. resolved bugs that would've evaporated 6M+ in under 5 minutes.",
        ),
    ];
    rows.iter()
        .map(|(date, company, role, bio)| Job {
            date: format!("{:<8}", date),
            company: company.to_string(),
            role: role.to_string(),
            bio: bio.to_string(),
        })
        .collect()
}

// Experience box geometry — matches the 80-col SUMMARY box (inner = 78 between
// the ║ borders; content rows reserve a 1-space margin each side → 76 of text).
const EXP_INNER: usize = 78;
const EXP_TEXT: usize = EXP_INNER - 2;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A full-width bordered row: `║ {html padded to EXP_TEXT} ║`.
fn exp_content_row(html: &str, visible: usize) -> String {
    format!(
        "<div class=\"exp-row\">║ {}{} ║</div>",
        html,
        " ".repeat(EXP_TEXT.saturating_sub(visible))
    )
}

/// The EXPERIENCE section as a single ASCII box (matching the other boxes) with
/// a `tree`-style, click-to-expand timeline. Each row is a complete `║ … ║` line,
/// and each job's bio rows live in a collapsible wrapper, so hiding them just
/// closes the box up without breaking the borders. Returns HTML — render `safe`.
pub fn experience_box() -> String {
    let mut s = String::new();
    let bar = "═".repeat(EXP_INNER);
    s.push_str(&format!("<div class=\"exp-row\">╔{bar}╗</div>"));
    // Centered title row.
    let title = "EXPERIENCE";
    let pad = EXP_INNER - title.len();
    let (l, r) = (pad / 2, pad - pad / 2);
    s.push_str(&format!(
        "<div class=\"exp-row\">║{}{}{}║</div>",
        " ".repeat(l),
        title,
        " ".repeat(r)
    ));
    s.push_str(&format!("<div class=\"exp-row\">╠{bar}╣</div>"));
    s.push_str(&exp_content_row(
        "<span class=\"exp-root\">~/career</span>",
        "~/career".width(),
    ));

    let jobs = experience();
    let last = jobs.len().saturating_sub(1);
    for (i, job) in jobs.iter().enumerate() {
        let conn = if i == last { "└─" } else { "├─" };
        // Head row (clickable).
        let head_plain = format!("{} {} {} · {} [+]", conn, job.date, job.company, job.role);
        let head_html = format!(
            "<span class=\"exp-conn\">{conn}</span> \
             <span class=\"exp-date\">{}</span> \
             <span class=\"exp-co\">{}</span> · \
             <span class=\"exp-role\">{}</span> \
             <span class=\"exp-toggle\">[+]</span>",
            esc(&job.date),
            esc(&job.company),
            esc(&job.role)
        );
        s.push_str("<div class=\"exp-job\">");
        s.push_str(&format!(
            "<button type=\"button\" class=\"exp-row exp-head\">║ {}{} ║</button>",
            head_html,
            " ".repeat(EXP_TEXT.saturating_sub(head_plain.width()))
        ));
        // Bio rows (collapsible). The bio is wrapped to leave room for the
        // 3-col branch prefix (`│  ` / `   `).
        let branch = if i == last { "   " } else { "│  " };
        s.push_str("<div class=\"exp-bio\">");
        for line in wrap_text(&job.bio, EXP_TEXT - 3) {
            let visible = 3 + line.width();
            s.push_str(&format!(
                "<div class=\"exp-row\">║ <span class=\"exp-conn\">{}</span>{}{} ║</div>",
                branch,
                esc(&line),
                " ".repeat(EXP_TEXT.saturating_sub(visible))
            ));
        }
        s.push_str("</div></div>");
    }

    s.push_str(&format!("<div class=\"exp-row\">╚{bar}╝</div>"));
    s
}

/// The truecolor half-block portrait (HTML) shown in the about sidebar.
/// Generated from the GitHub avatar by `cargo run --bin gen-avatar` and committed
/// to `templates/ascii/avatar.html`. Returns HTML (colored spans) — render with
/// the `safe` filter.
pub fn load_profile_art() -> String {
    std::fs::read_to_string("templates/ascii/avatar.html").unwrap_or_default()
}

/// Build a section box at one breakpoint. Blank lines are preserved; bullet
/// (`•`), quote (`>`), and link (`<a `) lines pass through untouched; everything
/// else is word-wrapped to `wrap_width`.
fn render_section(title: &str, body: &str, box_width: usize, wrap_width: usize) -> String {
    let mut out = String::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            out.push('\n');
        } else if line.contains('<') || line.starts_with('>') {
            // Lines with HTML (links, asm spans) pass through untouched —
            // word-wrapping them would split the tags.
            out.push_str(line);
            out.push('\n');
        } else if let Some(rest) = line.strip_prefix("• ") {
            // Bullets wrap with a hanging indent so long items fit narrow boxes.
            for (i, wrapped) in wrap_text(rest, wrap_width.saturating_sub(2))
                .iter()
                .enumerate()
            {
                out.push_str(if i == 0 { "• " } else { "  " });
                out.push_str(wrapped);
                out.push('\n');
            }
        } else {
            for wrapped in wrap_text(line, wrap_width) {
                out.push_str(&wrapped);
                out.push('\n');
            }
        }
    }
    create_header_box(title, out.trim_end_matches('\n'), box_width)
}

/// Build a wide section (main column) across all four breakpoints.
fn section(title: &str, body: &str) -> BoxSizes {
    BoxSizes {
        tiny: render_section(title, body, BOX_WIDTH_TINY, WRAP_WIDTH_TINY),
        small: render_section(title, body, BOX_WIDTH_SMALL, WRAP_WIDTH_SMALL),
        medium: render_section(title, body, BOX_WIDTH_MEDIUM, WRAP_WIDTH_MEDIUM),
        large: render_section(title, body, BOX_WIDTH_LARGE, WRAP_WIDTH_LARGE),
    }
}

/// Build a narrow section (sidebar) — same widths the blog sidebar uses, so the
/// boxes sit beside the main column instead of spanning the full width.
fn section_narrow(title: &str, body: &str) -> BoxSizes {
    BoxSizes {
        tiny: render_section(
            title,
            body,
            CATEGORIES_WIDTH_TINY,
            CATEGORIES_WIDTH_TINY - 4,
        ),
        small: render_section(
            title,
            body,
            CATEGORIES_WIDTH_SMALL,
            CATEGORIES_WIDTH_SMALL - 4,
        ),
        medium: render_section(
            title,
            body,
            CATEGORIES_WIDTH_MEDIUM,
            CATEGORIES_WIDTH_MEDIUM - 4,
        ),
        large: render_section(
            title,
            body,
            CATEGORIES_WIDTH_LARGE,
            CATEGORIES_WIDTH_LARGE - 4,
        ),
    }
}

pub fn build_about_boxes() -> AboutBoxes {
    let summary = "\
low level network software engineer
chaos computing enthusiast
professional button pusher
packet bender

San Francisco, California

<span class=\"asm-kw\">mov</span> <span class=\"asm-reg\">rax</span>, <span class=\"asm-str\">\"about me\"</span> <span class=\"asm-comment\">; ret</span>

self-taught since before i could legally drive. i live at the intersection of kernels, networking, firmware, and whatever rabbit hole i fell into this week. built an ISP, sold a company once. write Rust that would make most people uncomfortable.

daily-drive mainline kernels by choice, not accident. every device i own has run linux for the last 18 years. run BGP in production and think about it in the shower.

i never said i was stable. the code usually is though.";

    let skills = "\
top skills
• project management
• artificial intelligence (AI)
• algorithm development
• low-level / kernel / firmware
• networking & BGP
• Rust (uncomfortably so)

certifications
• Certified Boykisser License
• IC3 Digital Literacy Certification";

    let education = "\
Canyons Technical Education Center
computer engineering · 2016-2018

Salt Lake Community College
network engineering · 2023-2025";

    let links = "\
• github: <a href=\"https://github.com/vxfemboy\" target=\"_blank\" rel=\"noopener\">vxfemboy</a>
• x: <a href=\"https://x.com/vxfemboy\" target=\"_blank\" rel=\"noopener\">vxfemboy</a>
• linkedin: <a href=\"https://www.linkedin.com/in/vxfemboy\" target=\"_blank\" rel=\"noopener\">in/vxfemboy</a>
• email: <a href=\"mailto:zoa@zoa.sh\">zoa@zoa.sh</a>";

    AboutBoxes {
        summary_box: section("SUMMARY", summary),
        skills_box: section_narrow("SKILLS", skills),
        education_box: section_narrow("EDUCATION", education),
        links_box: section_narrow("LINKS", links),
    }
}
