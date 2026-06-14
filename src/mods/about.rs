//! Content for the `/about` page — Zoa's bio, rendered into responsive ASCII
//! boxes. Authored from the LinkedIn profile, kept in the site's playful voice.
//!
//! Each section is built at all four breakpoints via `create_header_box`, with
//! prose pre-wrapped to the matching `WRAP_WIDTH_*` (the box renderer splits on
//! `\n` but does not wrap on its own). Bullet/link lines are left intact.

use crate::mods::constants::*;
use crate::mods::{create_header_box, wrap_text, BoxSizes};

pub struct AboutBoxes {
    pub summary_box: BoxSizes,
    /// One bordered card per role (portfolio-style), in the main column.
    pub experience_cards: Vec<BoxSizes>,
    pub skills_box: BoxSizes,
    pub education_box: BoxSizes,
    pub links_box: BoxSizes,
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
        } else if line.contains("<a ") || line.starts_with('>') {
            // Link/quote lines pass through untouched (wrapping would split tags).
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

mov rax, \"about me\" ; ret

self-taught since before i could legally drive. i live at the intersection of kernels, networking, firmware, and whatever rabbit hole i fell into this week. built an ISP, sold a company once. write Rust that would make most people uncomfortable.

daily-drive mainline kernels by choice, not accident. every device i own has run linux for the last 18 years. run BGP in production and think about it in the shower.

i never said i was stable. the code usually is though.";

    // One card per role: (company title, "role · dates\n\nblurb").
    let experience: &[(&str, &str)] = &[
        (
            "FEMBOY CYBER NETWORKS",
            "founder · 2024-present\n\nbuilding an ISP that actually gives a damn about the people incumbents forgot. BGP peering, fiber, upstream wrangling. yes the name is real. yes the ASN is live.",
        ),
        (
            "DASH CRYSTAL",
            "research software engineer · 2026\n\napplied AI research: LLM fine-tuning (LoRA + multi-GPU full fine-tunes), eval harnesses, training-observability infra, systems instrumentation.",
        ),
        (
            "OCCAMSEC",
            "software engineer · 2023-2024\n\nfrontend + backend for InCenter, an automated breach-and-attack simulation platform. shipped the infra on AWS with terraform + ansible.",
        ),
        (
            "IPROYAL",
            "network software engineer · 2022-2023\n\nnetwork software engineering on proxy infrastructure at scale. yes i thought about packets constantly. yes that was fine.",
        ),
        (
            "STEALTH AI STARTUP",
            "founder & engineer · 2020-2023\n\nsolo-built an AI automation + marketing company. reverse-engineered platform APIs, ran stable diffusion pipelines before \"generative AI\" was a buzzword, conversational agents, hands-free multi-platform automation, automated payouts. mostly adult content creators -- pays better, problems are more interesting. scaled it solo until someone bought the whole thing. \"before it was cool.\"",
        ),
        (
            "FILMTEK CLOUD",
            "embedded linux engineer · 2021-2022\n\ncustom kernels, ported linux to embedded devices, kernel + socket level code, firewall/security software, hand-applied firmware patches. ran daily pentests, red/blue teams.",
        ),
        (
            "GOLDMAN SACHS",
            "unix sysadmin / IT engineer · 2021\n\ndomain user/group management, rewrote linux docs that hadn't been touched in years. resolved bugs that would've evaporated 6M+ in under 5 minutes.",
        ),
    ];

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
        experience_cards: experience
            .iter()
            .map(|(company, body)| section(company, body))
            .collect(),
        skills_box: section_narrow("SKILLS", skills),
        education_box: section_narrow("EDUCATION", education),
        links_box: section_narrow("LINKS", links),
    }
}
