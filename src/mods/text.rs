//! Terminal/ANSI rendering for `curl zoa.sh`.
//!
//! When a client sends a `curl`/`wget` User-Agent, the `curl_ansi` middleware
//! intercepts the rendered HTML response and converts it to ANSI-colored
//! plaintext using the site's own CSS for colors. Adapted from the previous
//! Rocket-based site (`zoa.sh_old/src/mods/text.rs`).

use actix_web::{
    body::{to_bytes, BoxBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    http::header::{HeaderValue, CONTENT_TYPE, USER_AGENT},
    middleware::Next,
    Error,
};
use csscolorparser::Color;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;

const CSS_PATH: &str = "static/css/main.css";
const RESET: &str = "\x1b[0m";

/// Selector → ANSI color-escape map, parsed once from the site CSS.
static CSS_STYLES: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let css = std::fs::read_to_string(CSS_PATH).unwrap_or_default();
    parse_css(&css)
});

/// Convert a CSS color string to a 24-bit ANSI foreground escape.
fn css_color_to_ansi(color: &str) -> String {
    if let Ok(parsed) = Color::from_html(color) {
        let rgba = parsed.to_rgba8();
        format!("\x1b[38;2;{};{};{}m", rgba[0], rgba[1], rgba[2])
    } else {
        // Default to light gray if color parsing fails.
        "\x1b[38;2;200;200;200m".to_string()
    }
}

/// Extract `color:` rules from CSS into a selector → ANSI map.
fn parse_css(css_content: &str) -> HashMap<String, String> {
    let mut styles = HashMap::new();
    let re = Regex::new(r"((?:\.|#|\w+)(?:[.#]\w+)*)\s*\{([^}]+)\}").unwrap();
    for cap in re.captures_iter(css_content) {
        let selector = cap[1].to_string();
        let properties = cap[2].to_string();
        if let Some(color) = properties
            .split(';')
            .find(|prop| prop.contains("color:") && !prop.contains("background"))
            .and_then(|prop| prop.split(':').nth(1))
            .map(|c| c.trim().to_string())
        {
            styles.insert(selector, css_color_to_ansi(&color));
        }
    }
    // Default text color for unmatched elements.
    styles.insert("default".to_string(), css_color_to_ansi("#00ff00"));
    styles
}

/// Render an HTML document body to ANSI-colored plaintext.
pub fn render_html_to_ansi(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut out = String::new();
    let body_selector = Selector::parse("body").unwrap();

    if let Some(body) = document.select(&body_selector).next() {
        format_element(&body, &CSS_STYLES, &mut out, 0);
    }

    // Collapse runs of 3+ blank lines down to 2 for tidier terminal output.
    let collapsed = Regex::new(r"\n{3,}").unwrap().replace_all(&out, "\n\n");
    format!("{}{}\n", collapsed.trim_end(), RESET)
}

fn style_for<'a>(
    element: &ElementRef,
    css_styles: &'a HashMap<String, String>,
    tag_name: &str,
) -> &'a str {
    element
        .value()
        .attr("class")
        .and_then(|class| {
            class
                .split_whitespace()
                .find_map(|c| css_styles.get(&format!(".{}", c)))
        })
        .or_else(|| css_styles.get(tag_name))
        .unwrap_or_else(|| css_styles.get("default").unwrap())
}

fn format_element(
    element: &ElementRef,
    css_styles: &HashMap<String, String>,
    output: &mut String,
    depth: usize,
) {
    let tag_name = element.value().name();

    // Skip decorative stars and duplicate responsive box variants: the page
    // emits the same box at four widths (.box-tiny/small/medium/large); for the
    // terminal we keep only the widest one.
    let classes = element.value().attr("class").unwrap_or("");
    if classes
        .split_whitespace()
        .any(|c| c == "star" || c == "box-tiny" || c == "box-small" || c == "box-medium")
    {
        return;
    }

    let style = style_for(element, css_styles, tag_name);
    let indent = "  ".repeat(depth);

    match tag_name {
        // Skip non-visual content entirely.
        "script" | "style" | "head" | "noscript" => {}

        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let text = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                let header_color = "\x1b[38;2;139;92;246m"; // purple
                output.push_str(&format!(
                    "\n{}{}\x1b[1m{}{}\n",
                    indent, header_color, text, RESET
                ));
            }
        }

        // ASCII-art boxes and code blocks: emit verbatim (preserve whitespace).
        "pre" => {
            let text = element.text().collect::<String>();
            let trimmed = text.trim_matches('\n');
            if !trimmed.trim().is_empty() {
                output.push_str(&format!("{}{}\n", style, trimmed));
                output.push_str(RESET);
                output.push('\n');
            }
        }

        "p" | "blockquote" | "dt" => {
            let text = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                let formatting = if tag_name == "blockquote" {
                    "\x1b[3m"
                } else {
                    ""
                };
                output.push_str(&format!(
                    "{}{}{}{}{}\n",
                    indent, style, formatting, text, RESET
                ));
            }
        }

        "ul" | "ol" => {
            for child in element.children() {
                if let Some(child) = ElementRef::wrap(child) {
                    if child.value().name() == "li" {
                        let content = child.text().collect::<String>().trim().to_string();
                        output.push_str(&format!("{}{}• {}{}\n", indent, style, content, RESET));
                    } else {
                        format_element(&child, css_styles, output, depth + 1);
                    }
                }
            }
        }

        "dd" => {
            if let Some(ul) = element.select(&Selector::parse("ul").unwrap()).next() {
                for li in ul.select(&Selector::parse("li").unwrap()) {
                    let content = li.text().collect::<String>().trim().to_string();
                    output.push_str(&format!("{}{}• {}{}\n", indent, style, content, RESET));
                }
            } else {
                let content = element.text().collect::<String>().trim().to_string();
                output.push_str(&format!("{}{}  {}{}\n", indent, style, content, RESET));
            }
        }

        "a" => {
            let text = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                output.push_str(&format!("{}\x1b[4m{}{}", style, text, RESET));
            }
        }

        "br" => output.push('\n'),

        // Recurse, emitting bare text nodes as we go.
        _ => {
            for node in element.children() {
                if let Some(child) = ElementRef::wrap(node) {
                    format_element(&child, css_styles, output, depth);
                } else if let Some(text) = node.value().as_text() {
                    let t = text.trim();
                    if !t.is_empty() {
                        output.push_str(&format!("{}{}{} ", style, t, RESET));
                    }
                }
            }
        }
    }
}

/// Middleware: rewrite `text/html` responses to ANSI plaintext for curl/wget.
pub async fn curl_ansi(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let wants_ansi = req
        .headers()
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|ua| {
            let ua = ua.to_lowercase();
            ua.contains("curl") || ua.contains("wget")
        })
        .unwrap_or(false);

    let res = next.call(req).await?;

    if !wants_ansi {
        return Ok(res.map_into_boxed_body());
    }

    let is_html = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|c| c.contains("text/html"))
        .unwrap_or(false);

    if !is_html {
        return Ok(res.map_into_boxed_body());
    }

    let (req, http_res) = res.into_parts();
    let (http_res, body) = http_res.into_parts();
    let bytes = to_bytes(body)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("failed to read response body"))?;
    let html = String::from_utf8_lossy(&bytes);
    let ansi = render_html_to_ansi(&html);

    let mut http_res = http_res.set_body(ansi);
    http_res.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    Ok(ServiceResponse::new(req, http_res).map_into_boxed_body())
}
