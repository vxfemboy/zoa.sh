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
            "building an ISP that actually gives a damn about the people incumbents forgot.\n\
             \n\
             BGP peering, fiber infrastructure, network engineering, upstream wrangling,\n\
             customer ops. yes the name is real. yes the ASN is live. yes the routing\n\
             tables are beautiful and i am very normal about them.\n\
             \n\
             ( ･`ω･´) building the internet of tomorrow ヽ(⌐■_■)ノ",
        ),
        (
            "2026",
            "dash crystal",
            "research software engineer",
            "applied AI research on the software engineering side. trained models, built the infrastructure to train and measure them, and instrumented existing systems to squeeze research data out of them.\n\
             \n\
             what i shipped:\n\
             \n\
             LLM fine-tuning - supervised fine-tuning of large language models for structured, domain-specific tasks. curriculum-based data generation, parameter-efficient training (LoRA) on single GPUs and full fine-tunes distributed across multi-GPU clusters. wrote the eval harnesses to actually verify the models learned what they were supposed to.\n\
             \n\
             ML training infrastructure - internal training-observability tooling: thread-safe metric tracking, distributed-aware aggregation, anomaly detection, low overhead by design. strict typing, high coverage, CI. shipped.\n\
             systems instrumentation - research-grade logging bolted into existing high-performance systems to quantify where the work was actually happening, without touching their behavior.\n\
             \n\
             infra - self-hosted internal services and deployment automation.\n\
             \n\
             // research is just breaking things on purpose and writing down what fell out.",
        ),
        (
            "2023–24",
            "occamsec",
            "software engineer",
            "frontend and backend dev for InCenter — an automated breach and attack\n\
             simulation platform covering web, network, and cloud security.\n\
             \n\
             deployed and configured InCenter infra on AWS using terraform and ansible.\n\
             \n\
             // the job where breaking things was the actual job description.",
        ),
        (
            "2022–23",
            "iproyal",
            "network software engineer",
            "network software engineering on proxy infrastructure at scale.\n\
             yes i thought about packets constantly. yes that was fine.",
        ),
        (
            "2020–23",
            "stealth AI startup",
            "founder & engineer",
            "solo founded and ran an AI automation and marketing company. just me. built everything,\n\
             sold everything, supported everything.\n\
             \n\
             most of my clients were adult content creators was on OnlyFans, Fansly, Twitter,\n\
             Tumblr, Cam Sites, etc that whole world. also had mainstream social media growth clients and social media marketing agencies work with me to help their customers (YouTube, Instagram, Spotify, Discord, the usual). the adult industry was the bulk of it.\n\
             \n\
             honestly the adult entertainment industry pays better and the technical problems are more\n\
             interesting.\n\
             \n\
             what i shipped:\n\
             \n\
             - stable diffusion pipelines + custom AI image editing tooling,\n\
               you know, before \"generative AI\" was a buzzword anyone used\n\
             - conversational AI agents handling inbound DMs and fan interactions\n\
             \n\
             - full platform automation: posting, scheduling, shares, reposts,\n\
               likes, replies, follows running simultaneously across Twitter,\n\
               Instagram, Tumblr and more, hands-free\n\
             - automated transaction handling fan pays, gets acknowledged,\n\
               money routes to the creator, zero manual intervention\n\
             - audience targeting and growth systems that turned one-time\n\
               followers into recurring paying subscribers for my clients\n\
             - the whole stack was designed so clients checked a dashboard\n\
               occasionally and otherwise forgot it existed\n\
             \n\
             retention was the metric that mattered. clients stayed because their own subscriber bases kept growing and paying.\n\
             recurring revenue on both ends.\n\
             \n\
             ran it all solo. customer acquisition, support, dev, infra, billing\n\
             \n\
             scaled it to the point someone made an offer on the whole thing: software, IP, client relationships, resources, everything.\n\
             asset acquisition.\n\
             NDA covers buyer identity and terms.\n\
             \n\
             // reverse engineered platform APIs, built thin clients for major social and messaging services, ran adult content automation AI infrastructure at scale. from my bed.\n\
             \"before it was cool.\"",
        ),
        (
            "2021–22",
            "filmtek cloud",
            "embedded linux engineer",
            "built and compiled custom kernels. ported linux to embedded devices.\n\
             wrote kernel-level and socket-level code. developed and modified firewall\n\
             and security software. custom linux firmware with hand-applied patches.\n\
             \n\
             racked servers, NAS, switches. configured DDNS, DHCP, name servers.\n\
             debugged hardware issues. ran daily internal and external pentests.\n\
             organized red and blue teams on the company network. wrote security\n\
             reports and patching plans. automated scanning of new database leaks\n\
             for employee records. hired and trained new employees.\n\
             \n\
             // if it compiles it ships. learned why that's a bad philosophy here.",
        ),
        (
            "2021",
            "goldman sachs",
            "unix sysadmin / IT engineer",
            "domain user and group management. rewrote linux and unix documentation\n\
             that hadn't been touched in years.\n\
             \n\
             // the documentation was basically archaeology\n\
             \n\
             resolved undocumented server issues\n\
             for clients and engineers. fixed ICA remote client connectivity.\n\
             sorted out aruba router, VOIP, and firewall configs.\n\
             rebooted the rdps;\n\
             \n\
             // resolved bugs that would have led to over 6 million in investments to disapear in under 5 minutes",
        ),
        (
            "feb 21",
            "foxconn",
            "server engineer",
            "test servers, storage, switches and sub-assemblies.\n\
             verify proper components are received per order pick list prior to build.\n\
             scan all components into the manufacturing shop floor system to ensure proper parts and inventory tracking.\n\
             use online methods and work instructions to build racks per customer specification.",
        ),
        (
            "2020–21",
            "pizzeria limone",
            "software engineer",
            "built internal tooling to automate analytics and invoicing.\n\
             found an active RCE exploit a former employee had left running. killed it.\n\
             debugged a broken SMTP server. built webservers for employee training,\n\
             interview management, and onboarding.\n\
             \n\
             // did incident response at a pizza place. it counted.",
        ),
        (
            "2019",
            "herbs for health",
            "tech support",
            "network monitoring, security config, access permissions, diagnosing\n\
             networking problems with diagnostic tooling, routine maintenance.\n\
             \n\
             // early IT. everyone starts somewhere.",
        ),
        (
            "2014–18",
            "canyons school district",
            "IT support",
            "maintained schools networks, internally and externally tested and reported security flaws and vulnerabilities district wide, fixed teachers and student machines, set up classroom networks and devices.\n\
             \n\
             \n\
             // tldr: i got into 'ethical hacking' by finding holes in networks i was supposed to be using to do homework on ;3\n\
             // classic origin story",
        ),
        (
            "2016–17",
            "nickelcade",
            "repair tech",
            "fixed arcade and vending machines. soldering, wiring diagrams, replacing\n\
             mechanical and electrical parts. diagnosed malfunctions, ordered parts,\n\
             handled billing.\n\
             \n\
             // this is where i learned that hardware is just software you can hit",
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

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Word-wrap each newline-separated paragraph; blank lines become empty rows.
fn wrap_bio_paragraphs(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for chunk in text.split('\n') {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(wrap_text(trimmed, max_width));
        }
    }
    lines
}

fn push_bio_rows(
    s: &mut String,
    branch: &str,
    text: usize,
    lines: &[String],
    pclass: Option<&str>,
) {
    let width = text - 3;
    for line in lines {
        if line.is_empty() {
            s.push_str(&format!(
                "<div class=\"exp-row\">║ <span class=\"exp-conn\">{}</span>{} ║</div>",
                branch,
                " ".repeat(width)
            ));
            continue;
        }
        let visible = 3 + line.width();
        let body = match pclass {
            Some(cls) => format!("<span class=\"{}\">{}</span>", cls, esc(line)),
            None => esc(line),
        };
        s.push_str(&format!(
            "<div class=\"exp-row\">║ <span class=\"exp-conn\">{}</span>{}{} ║</div>",
            branch,
            body,
            " ".repeat(text.saturating_sub(visible))
        ));
    }
}

/// A full-width bordered row: `║ {html padded to text} ║` (text = inner - 2,
/// the 1-space margin each side).
fn exp_content_row(html: &str, visible: usize, text: usize) -> String {
    format!(
        "<div class=\"exp-row\">║ {}{} ║</div>",
        html,
        " ".repeat(text.saturating_sub(visible))
    )
}

/// The EXPERIENCE section as an ASCII box (matching the other boxes) with a
/// `tree`-style, click-to-expand timeline. Each row is a complete `║ … ║` line,
/// and each job's bio rows live in a collapsible wrapper, so hiding them just
/// closes the box up without breaking the borders. Returns HTML — render `safe`.
///
/// `inner` is the column count between the ║ borders (box width = inner + 2).
/// When `compact` (the narrow mobile variant) the head shows just date + company
/// and the role is folded into the bio so the line fits a ~40-col box.
pub fn experience_box(inner: usize, compact: bool) -> String {
    let text = inner - 2;
    let bar = "═".repeat(inner);
    let mut s = String::new();
    s.push_str(&format!("<div class=\"exp-row\">╔{bar}╗</div>"));
    // Centered title row.
    let title = "EXPERIENCE";
    let pad = inner - title.len();
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
        text,
    ));

    let jobs = experience();
    let last = jobs.len().saturating_sub(1);
    for (i, job) in jobs.iter().enumerate() {
        let conn = if i == last { "└─" } else { "├─" };
        // Head row (clickable). Compact drops the role from the head line.
        let (head_plain, head_html) = if compact {
            (
                format!("{} {} {} [+]", conn, job.date, job.company),
                format!(
                    "<span class=\"exp-conn\">{conn}</span> \
                     <span class=\"exp-date\">{}</span> \
                     <span class=\"exp-co\">{}</span> \
                     <span class=\"exp-toggle\">[+]</span>",
                    esc(&job.date),
                    esc(&job.company),
                ),
            )
        } else {
            (
                format!("{} {} {} · {} [+]", conn, job.date, job.company, job.role),
                format!(
                    "<span class=\"exp-conn\">{conn}</span> \
                     <span class=\"exp-date\">{}</span> \
                     <span class=\"exp-co\">{}</span> · \
                     <span class=\"exp-role\">{}</span> \
                     <span class=\"exp-toggle\">[+]</span>",
                    esc(&job.date),
                    esc(&job.company),
                    esc(&job.role)
                ),
            )
        };
        s.push_str("<div class=\"exp-job\">");
        s.push_str(&format!(
            "<button type=\"button\" class=\"exp-row exp-head\">║ {}{} ║</button>",
            head_html,
            " ".repeat(text.saturating_sub(head_plain.width()))
        ));
        // Bio rows (collapsible), wrapped to leave room for the 3-col branch
        // prefix. In compact mode the role leads the bio (it's not on the head),
        // as its own colored paragraph separated from the description.
        let branch = if i == last { "   " } else { "│  " };
        let paragraphs: Vec<(&str, Option<&str>)> = if compact {
            vec![
                (job.role.as_str(), Some("exp-role")),
                (job.bio.as_str(), None),
            ]
        } else {
            vec![(job.bio.as_str(), None)]
        };
        s.push_str("<div class=\"exp-bio\">");
        for (pi, (ptext, pclass)) in paragraphs.iter().enumerate() {
            if pi > 0 {
                // Blank branch row between paragraphs.
                s.push_str(&format!(
                    "<div class=\"exp-row\">║ <span class=\"exp-conn\">{}</span>{} ║</div>",
                    branch,
                    " ".repeat(text - 3)
                ));
            }
            push_bio_rows(
                &mut s,
                branch,
                text,
                &wrap_bio_paragraphs(ptext, text - 3),
                *pclass,
            );
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

/// Build a wide section (main column) across all four breakpoints. Shared with
/// the projects page. Body lines containing HTML (links) pass through untouched;
/// prose wraps; `• ` bullets get a hanging indent.
pub(crate) fn section(title: &str, body: &str) -> BoxSizes {
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

    let links = "\
• github: <a href=\"https://github.com/vxfemboy\" target=\"_blank\" rel=\"noopener\">vxfemboy</a>
• x: <a href=\"https://x.com/vxfemboy\" target=\"_blank\" rel=\"noopener\">vxfemboy</a>
• linkedin: <a href=\"https://www.linkedin.com/in/vxfemboy\" target=\"_blank\" rel=\"noopener\">in/vxfemboy</a>
• email: <a href=\"mailto:zoa@zoa.sh\">zoa@zoa.sh</a>

• uses: <a href=\"/uses\">/uses</a>
• now: <a href=\"/now\">/now</a>";

    AboutBoxes {
        summary_box: section("SUMMARY", summary),
        skills_box: section_narrow("SKILLS", skills),
        links_box: section_narrow("LINKS", links),
    }
}
