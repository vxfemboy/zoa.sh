//! Markdown blog post loader
//!
//! Automatically loads and parses .md files from the posts/ directory.
//! Supports YAML frontmatter for metadata (title, date, slug).
//! Includes syntax highlighting for code blocks.

use std::fs;
use std::path::Path;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashSet;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

/// A parsed blog post
#[derive(Debug, Clone)]
pub struct MarkdownPost {
    pub title: String,
    pub date: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub content_html: String,
    pub content_plain: String,
    /// Optional per-post OG/social image, a path under `posts/assets/`
    /// (frontmatter `social:`). None → the default site image.
    #[allow(dead_code)]
    pub social: Option<String>,
}

/// Syntax highlighter using syntect
struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    fn highlight(&self, code: &str, lang: &str) -> String {
        // Try to find syntax for the language
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use a dark theme that fits the site aesthetic
        let theme = &self.theme_set.themes["base16-ocean.dark"];

        match highlighted_html_for_string(code, &self.syntax_set, syntax, theme) {
            Ok(html) => html,
            Err(_) => format!("<pre><code>{}</code></pre>", html_escape(code)),
        }
    }
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Parse YAML-style frontmatter from markdown content
/// Returns (frontmatter_map, remaining_content)
fn parse_frontmatter(content: &str) -> (std::collections::HashMap<String, String>, &str) {
    let mut map = std::collections::HashMap::new();

    let content = content.trim_start();
    if !content.starts_with("---") {
        return (map, content);
    }

    // Find the closing ---
    let after_first = &content[3..];
    if let Some(end_idx) = after_first.find("\n---") {
        let frontmatter = &after_first[..end_idx];
        let remaining = &after_first[end_idx + 4..].trim_start();

        // Parse simple key: value pairs
        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(colon_idx) = line.find(':') {
                let key = line[..colon_idx].trim().to_lowercase();
                let value = line[colon_idx + 1..].trim().to_string();
                map.insert(key, value);
            }
        }

        return (map, remaining);
    }

    (map, content)
}

fn heading_num(level: HeadingLevel) -> u32 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Ensure a slug is unique within one document (append -2, -3, … on collision).
fn unique_slug(base: &str, used: &mut HashSet<String>) -> String {
    let base = if base.is_empty() {
        "section".to_string()
    } else {
        base.to_string()
    };
    if used.insert(base.clone()) {
        return base;
    }
    let mut n = 2;
    loop {
        let cand = format!("{base}-{n}");
        if used.insert(cand.clone()) {
            return cand;
        }
        n += 1;
    }
}

/// Convert markdown to HTML with syntax highlighting for code blocks
fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);

    let highlighter = SyntaxHighlighter::new();
    let parser = Parser::new_ext(markdown, options);

    let mut html_output = String::new();
    let mut code_block_lang: Option<String> = None;
    let mut code_block_content = String::new();
    let mut heading: Option<HeadingLevel> = None;
    let mut heading_html = String::new();
    let mut heading_text = String::new();
    let mut used_slugs: HashSet<String> = HashSet::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                // Extract language from fenced code block
                code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.as_ref().trim();
                        if lang_str.is_empty() {
                            None
                        } else {
                            Some(lang_str.to_string())
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
                code_block_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                // Apply syntax highlighting
                let lang = code_block_lang.as_deref().unwrap_or("txt");
                let highlighted = highlighter.highlight(&code_block_content, lang);
                html_output.push_str(&highlighted);
                code_block_lang = None;
                code_block_content.clear();
            }
            Event::Text(text) if code_block_lang.is_some() || !code_block_content.is_empty() => {
                // Inside a code block - accumulate content
                code_block_content.push_str(&text);
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(level);
                heading_html.clear();
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                let lvl = heading.take().map(heading_num).unwrap_or(1);
                if lvl >= 2 {
                    let slug = unique_slug(&slugify(&heading_text), &mut used_slugs);
                    html_output.push_str(&format!(
                        "<h{lvl} id=\"{slug}\">{heading_html}<a class=\"heading-anchor\" href=\"#{slug}\" aria-label=\"link to this section\">#</a></h{lvl}>"
                    ));
                } else {
                    html_output.push_str(&format!("<h{lvl}>{heading_html}</h{lvl}>"));
                }
            }
            ev if heading.is_some() => {
                if let Event::Text(ref t) = ev {
                    heading_text.push_str(t);
                } else if let Event::Code(ref c) = ev {
                    heading_text.push_str(c);
                }
                push_html_event(&mut heading_html, ev);
            }
            Event::Text(text) if code_block_lang.is_none() && code_block_content.is_empty() => {
                // Normal text outside code blocks
                push_html_event(&mut html_output, Event::Text(text));
            }
            other => {
                if code_block_lang.is_none() && code_block_content.is_empty() {
                    push_html_event(&mut html_output, other);
                }
            }
        }
    }

    let html = colorize_comments(&html_output);
    // Post images are authored `src="assets/…"` (relative). Make them
    // root-absolute so they load whether the post is at /post/<slug> or served
    // at a vanity domain's root (namecheap.wtf). Handle double-quoted,
    // single-quoted, and bare/unquoted `src=assets/…` forms.
    html.replace("src=\"assets/", "src=\"/post/assets/")
        .replace("src='assets/", "src='/post/assets/")
        .replace("src=assets/", "src=/post/assets/")
}

/// Tag paragraphs that begin with `//` as code-style comments so they render in
/// the dimmed comment color (the author-voice `// ...` asides). Only whole
/// paragraphs are matched, so `//` inside code blocks or URLs is untouched.
fn colorize_comments(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(idx) = rest.find("<p>//") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + 3..]; // skip past "<p>"
        if let Some(end) = after.find("</p>") {
            out.push_str("<p class=\"md-comment\">");
            out.push_str(&after[..end]);
            out.push_str("</p>");
            rest = &after[end + 4..];
        } else {
            // Unterminated paragraph; emit the rest unchanged.
            out.push_str(&rest[idx..]);
            rest = "";
            break;
        }
    }

    out.push_str(rest);
    out
}

/// Push a single pulldown-cmark event to HTML output
fn push_html_event(output: &mut String, event: Event) {
    let events = std::iter::once(event);
    pulldown_cmark::html::push_html(output, events);
}

/// Strip HTML tags and get plain text (for ASCII box display)
fn html_to_plain_text(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
            }
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => {
                result.push(ch);
            }
            _ => {}
        }
    }

    // Decode common HTML entities
    result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        // Clean up excessive newlines
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// URL-safe slug: lowercase, `[a-z0-9]` kept, everything else collapsed to `-`,
/// with leading/trailing dashes trimmed.
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Generate a slug from the filename or title
fn generate_slug(filename: &str, title: &str) -> String {
    // Prefer filename-based slug (without .md extension)
    let slug = filename.trim_end_matches(".md");
    if !slug.is_empty() && slug != filename {
        return slug.to_string();
    }

    // Fall back to title-based slug
    slugify(title)
}

/// Load a single markdown file
pub fn load_markdown_file(path: &Path) -> Option<MarkdownPost> {
    let content = fs::read_to_string(path).ok()?;
    let filename = path.file_name()?.to_str()?;

    let (frontmatter, markdown_content) = parse_frontmatter(&content);

    let title = frontmatter
        .get("title")
        .cloned()
        .unwrap_or_else(|| filename.trim_end_matches(".md").to_string());

    let date = frontmatter
        .get("date")
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());

    let slug = frontmatter
        .get("slug")
        .cloned()
        .unwrap_or_else(|| generate_slug(filename, &title));

    // Parse tags (comma-separated or single value)
    let tags: Vec<String> = frontmatter
        .get("tags")
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let social = frontmatter
        .get("social")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let content_html = markdown_to_html(markdown_content);
    let content_plain = html_to_plain_text(&content_html);

    Some(MarkdownPost {
        title,
        date,
        slug,
        tags,
        content_html,
        content_plain,
        social,
    })
}

/// Load all markdown posts from the posts/ directory
pub fn load_all_posts(posts_dir: &str) -> Vec<MarkdownPost> {
    let path = Path::new(posts_dir);

    if !path.exists() || !path.is_dir() {
        tracing::warn!("Posts directory not found: {}", posts_dir);
        return Vec::new();
    }

    let mut posts: Vec<MarkdownPost> = fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "md")
                .unwrap_or(false)
        })
        .filter_map(|entry| load_markdown_file(&entry.path()))
        .collect();

    // Sort by date (newest first)
    posts.sort_by(|a, b| b.date.cmp(&a.date));

    tracing::info!("Loaded {} markdown posts", posts.len());
    posts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
title: Test Post
date: 2024-12-28
slug: test-post
---

# Hello World

This is content.
"#;

        let (fm, remaining) = parse_frontmatter(content);
        assert_eq!(fm.get("title"), Some(&"Test Post".to_string()));
        assert_eq!(fm.get("date"), Some(&"2024-12-28".to_string()));
        assert!(remaining.contains("# Hello World"));
    }

    #[test]
    fn test_markdown_to_html() {
        let md = "**bold** and *italic*";
        let html = markdown_to_html(md);
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }
}
