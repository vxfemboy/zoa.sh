//! Downloadable resume (PDF + DOCX), generated at request time from the same
//! `about::experience()` data that drives `/about`. Content is trimmed for
//! resume use: inline `//` asides and joke entries are stripped, but the
//! underlying facts are the single source of truth shared with the web page.
//!
//! Identity is pinned to the canonical zoa.sh persona regardless of which
//! domain the request came in on — a resume shouldn't fork per-vanity-domain.

use crate::mods::about::{experience, Job};
use crate::mods::site;
use docx_rs::{AlignmentType, Docx, Paragraph, Run, RunFonts};
use printpdf::*;
use std::io::{BufWriter, Cursor};

const NAME: &str = "Zoa Hickenlooper";
const TAGLINE: &str = "Low-level network software engineer — kernels, BGP, firmware, Rust";
const ACCENT: (f32, f32, f32) = (0.2, 0.667, 1.0); // site's #33aaff

struct ResumeData {
    email: String,
    links: Vec<(&'static str, String)>,
    summary: &'static str,
    skills: Vec<&'static str>,
    jobs: Vec<Job>,
}

/// Strip `//` asides, decorative quote-only lines, and collapse blank runs.
/// Caps each job to a resume-reasonable number of lines so a 12-role career
/// history doesn't sprawl across ten pages.
fn trim_bio(bio: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in bio.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("//") {
            continue;
        }
        if t.starts_with('"') && t.ends_with('"') {
            continue;
        }
        let normalized = if let Some(rest) = t.strip_prefix("- ") {
            format!("• {}", rest)
        } else {
            t.to_string()
        };
        out.push(normalized);
        if out.len() >= 8 {
            break;
        }
    }
    out
}

fn build_data() -> ResumeData {
    let site = site::resolve("zoa.sh");
    let jobs: Vec<Job> = experience()
        .into_iter()
        .map(|j| Job {
            date: j.date.trim().to_string(),
            company: j.company,
            role: j.role,
            bio: trim_bio(&j.bio).join("\n"),
        })
        .collect();

    ResumeData {
        email: site.email.clone(),
        links: vec![
            ("github", "github.com/vxfemboy".to_string()),
            ("linkedin", "linkedin.com/in/vxfemboy".to_string()),
            ("web", "zoa.sh".to_string()),
        ],
        summary: "Self-taught systems engineer working across kernels, networking, and \
                   firmware. Founded and operate an ISP (live BGP, fiber infrastructure, \
                   upstream peering). Built and sold an AI automation company solo — \
                   product, infra, support, and billing. Writes Rust professionally and \
                   for fun.",
        skills: vec![
            "Rust",
            "Networking & BGP",
            "Low-level / kernel / firmware",
            "AI / ML engineering",
            "Algorithm design",
            "Project management",
        ],
        jobs,
    }
}

fn mm_page_height() -> f32 {
    297.0
}

struct PdfWriter {
    doc: PdfDocumentReference,
    layer: PdfLayerReference,
    font: IndirectFontRef,
    font_bold: IndirectFontRef,
    y: f32,
    margin: f32,
    width: f32,
}

impl PdfWriter {
    fn new() -> Self {
        let (doc, page, layer) =
            PdfDocument::new("Zoa Hickenlooper - Resume", Mm(210.0), Mm(297.0), "Layer 1");
        let font = doc.add_builtin_font(BuiltinFont::Courier).unwrap();
        let font_bold = doc.add_builtin_font(BuiltinFont::CourierBold).unwrap();
        let layer_ref = doc.get_page(page).get_layer(layer);
        Self {
            doc,
            layer: layer_ref,
            font,
            font_bold,
            y: mm_page_height() - 18.0,
            margin: 18.0,
            width: 210.0 - 36.0,
        }
    }

    fn ensure_room(&mut self, needed: f32) {
        if self.y - needed < self.margin {
            let (page, layer) = self.doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
            self.layer = self.doc.get_page(page).get_layer(layer);
            self.y = mm_page_height() - 18.0;
        }
    }

    fn text(&mut self, s: &str, size: f32, bold: bool, r: f32, g: f32, b: f32) {
        self.ensure_room(size / 2.2);
        self.layer
            .set_fill_color(Color::Rgb(Rgb::new(r, g, b, None)));
        let font = if bold { &self.font_bold } else { &self.font };
        self.layer
            .use_text(s, size, Mm(self.margin), Mm(self.y), font);
        self.y -= size / 2.2;
    }

    fn rule(&mut self) {
        self.ensure_room(4.0);
        self.layer
            .set_outline_color(Color::Rgb(Rgb::new(ACCENT.0, ACCENT.1, ACCENT.2, None)));
        self.layer.set_outline_thickness(0.6);
        let line = Line {
            points: vec![
                (Point::new(Mm(self.margin), Mm(self.y)), false),
                (Point::new(Mm(self.margin + self.width), Mm(self.y)), false),
            ],
            is_closed: false,
        };
        self.layer.add_line(line);
        self.y -= 4.0;
    }

    fn heading(&mut self, s: &str) {
        self.ensure_room(10.0);
        self.text(s, 13.0, true, ACCENT.0, ACCENT.1, ACCENT.2);
        self.rule();
    }

    fn gap(&mut self, mm: f32) {
        self.y -= mm;
    }

    /// Courier glyphs are exactly 0.6em wide, so the character budget for a
    /// line is derived from the actual usable width rather than guessed.
    fn char_budget(&self, size: f32) -> usize {
        let width_pt = self.width * 2.834_645_7;
        (width_pt / (size * 0.6)) as usize
    }

    /// Word-wrap to fit the page width at this font size and emit each line.
    fn wrapped(&mut self, s: &str, size: f32) {
        let max_chars = self.char_budget(size);
        for raw_line in s.lines() {
            if raw_line.trim().is_empty() {
                self.gap(size / 3.0);
                continue;
            }
            let mut cur = String::new();
            for word in raw_line.split_whitespace() {
                if !cur.is_empty() && cur.len() + 1 + word.len() > max_chars {
                    self.text(&cur, size, false, 0.15, 0.15, 0.18);
                    cur.clear();
                }
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(word);
            }
            if !cur.is_empty() {
                self.text(&cur, size, false, 0.15, 0.15, 0.18);
            }
        }
    }

    fn finish(self) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = BufWriter::new(&mut buf);
            self.doc.save(&mut writer).unwrap();
        }
        buf
    }
}

pub fn render_pdf() -> Vec<u8> {
    let data = build_data();
    let mut w = PdfWriter::new();

    w.text(NAME, 22.0, true, 0.05, 0.05, 0.08);
    w.text(TAGLINE, 10.5, false, 0.35, 0.35, 0.38);
    let contact = format!(
        "{}  ·  {}  ·  {}  ·  {}",
        data.email, data.links[0].1, data.links[1].1, data.links[2].1
    );
    w.text(&contact, 9.5, false, 0.35, 0.35, 0.38);
    w.gap(4.0);

    w.heading("SUMMARY");
    w.wrapped(data.summary, 10.0);
    w.gap(3.0);

    w.heading("SKILLS");
    w.wrapped(&data.skills.join("   •   "), 10.0);
    w.gap(3.0);

    w.heading("EXPERIENCE");
    for job in &data.jobs {
        w.ensure_room(14.0);
        w.text(
            &format!("{} — {}", job.company, job.role),
            11.0,
            true,
            0.05,
            0.05,
            0.08,
        );
        w.text(&job.date, 9.0, false, 0.45, 0.45, 0.48);
        w.wrapped(&job.bio, 9.5);
        w.gap(3.5);
    }

    w.finish()
}

pub fn render_docx() -> Vec<u8> {
    let data = build_data();

    let heading = |text: &str| {
        Paragraph::new()
            .add_run(
                Run::new()
                    .add_text(text)
                    .bold()
                    .size(28)
                    .fonts(RunFonts::new().ascii("Consolas")),
            )
            .align(AlignmentType::Left)
    };
    let body = |text: &str| Paragraph::new().add_run(Run::new().add_text(text).size(21));

    let mut docx = Docx::new()
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(NAME).bold().size(44))
                .align(AlignmentType::Left),
        )
        .add_paragraph(body(TAGLINE))
        .add_paragraph(body(&format!(
            "{}  |  {}  |  {}  |  {}",
            data.email, data.links[0].1, data.links[1].1, data.links[2].1
        )))
        .add_paragraph(Paragraph::new())
        .add_paragraph(heading("SUMMARY"))
        .add_paragraph(body(data.summary))
        .add_paragraph(Paragraph::new())
        .add_paragraph(heading("SKILLS"))
        .add_paragraph(body(&data.skills.join("  •  ")))
        .add_paragraph(Paragraph::new())
        .add_paragraph(heading("EXPERIENCE"));

    for job in &data.jobs {
        docx = docx
            .add_paragraph(Paragraph::new().add_run(
                Run::new()
                    .add_text(format!("{} — {}", job.company, job.role))
                    .bold()
                    .size(23),
            ))
            .add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&job.date).italic().size(19)),
            );
        for line in job.bio.lines() {
            docx = docx.add_paragraph(body(line));
        }
        docx = docx.add_paragraph(Paragraph::new());
    }

    let mut cursor = Cursor::new(Vec::new());
    docx.build().pack(&mut cursor).unwrap();
    cursor.into_inner()
}
