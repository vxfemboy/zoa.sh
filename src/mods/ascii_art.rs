use crate::mods::data::NavItem;
use unicode_width::UnicodeWidthChar;

// Unified builder for ASCII boxes
#[derive(Clone, Copy)]
pub enum BoxStyle {
    Header,
    Footer,
    AboutWithAscii,
}

pub struct BoxBuilder<'a> {
    title: Option<&'a str>,
    content: Option<&'a str>,
    ascii_art: Option<&'a str>,
    width: usize,
    style: BoxStyle,
}

impl<'a> BoxBuilder<'a> {
    pub fn new() -> Self {
        Self {
            title: None,
            content: None,
            ascii_art: None,
            width: 40,
            style: BoxStyle::Header,
        }
    }

    pub fn with_title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_content(mut self, content: &'a str) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_ascii_art(mut self, ascii_art: &'a str) -> Self {
        self.ascii_art = Some(ascii_art);
        self
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    pub fn with_style(mut self, style: BoxStyle) -> Self {
        self.style = style;
        self
    }

    pub fn build(self) -> String {
        match self.style {
            BoxStyle::Header => {
                let title = self.title.unwrap_or("");
                let content = self.content.unwrap_or("");
                create_header_box(title, content, self.width)
            }
            BoxStyle::Footer => {
                let content = self.content.unwrap_or("");
                create_footer_box(content, self.width)
            }
            BoxStyle::AboutWithAscii => {
                let title = self.title.unwrap_or("");
                let content = self.content.unwrap_or("");
                let ascii = self.ascii_art.unwrap_or("");
                create_about_box_with_ascii(title, content, ascii, self.width)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Unicode & Emoji Handling Strategy for ASCII Art
// ═══════════════════════════════════════════════════════════════════════════
//
// The Challenge:
// - Unicode characters (especially emoji) render at different widths across browsers/OS
// - CSS `unicode-width` reports standard widths, but actual rendering varies by font
// - There is NO perfect server-side solution (see: https://nolanlawson.com/2022/04/08/)
//
// Our Solution (Web Search Best Practices):
// 1. ✅ Use monospaced font stack (see templates/index.html.tera)
// 2. ✅ Use unicode-width crate for 99% of characters (automatic)
// 3. ✅ Manual overrides for problematic characters (get_actual_char_width)
// 4. ✅ Support fractional widths (1.5, 1.3, etc.)
// 5. ✅ Skip HTML tags but count rendered elements like <img>
//
// How to Add New Emoji:
// 1. Add it to your content (e.g., footer, blog post)
// 2. Build and test in YOUR browser
// 3. If borders misalign, add override in get_actual_char_width()
// 4. Adjust value (try: 1.0, 1.3, 1.5, 1.8, 2.0) until perfect
//
// Note: Different browsers may need different values. Optimize for your main audience.
// ═══════════════════════════════════════════════════════════════════════════

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        let visual_width = count_visual_width_excluding_html(&test_line);

        if visual_width <= max_width {
            current_line = test_line;
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static EMOJI_MEMO: Lazy<Mutex<HashMap<char, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn replace_problematic_chars(text: &str) -> String {
    // Automatically replace emojis and special Unicode chars that don't render
    // consistently in monospace fonts

    let mut result = String::new();

    for c in text.chars() {
        let codepoint = c as u32;

        // Check if character is problematic (emoji or special Unicode)
        let is_problematic = matches!(codepoint,
            // Emoji ranges (consolidated to avoid overlaps)
            0x1F100..=0x1F9FF |  // All emoji blocks: Enclosed Alphanumeric Supplement through Supplemental Symbols
            0x2600..=0x27BF |    // Misc symbols + Dingbats (consolidated)
            0x2B00..=0x2BFF |    // Miscellaneous Symbols and Arrows (includes ⭐)
            0x2460..=0x24FF |    // Enclosed Alphanumerics
            0xFE0F |             // Variation Selector-16
            0x200D |             // Zero Width Joiner
            0x200B..=0x200F |    // Zero-width spaces
            0xFEFF               // Zero Width No-Break Space
        );

        if is_problematic {
            // Memoize conversion per char
            if let Ok(memo) = EMOJI_MEMO.lock() {
                if let Some(cached) = memo.get(&c) {
                    result.push_str(cached);
                    continue;
                }
            }
            // Handle specific symbols
            match c {
                // Zero-width characters - remove completely
                '\u{fe0f}' | '\u{200d}' | '\u{200b}' | '\u{200c}' | '\u{200f}' | '\u{feff}' => {}

                // Special: Use custom copyleft SVG
                '🄯' => {
                    result.push_str(
                        "<img src=\"/static/copyleft.svg\" alt=\"copyleft\" class=\"emoji-img\">",
                    );
                }

                // All other emojis/special chars: convert to SVG using Twemoji
                // Each SVG is styled to take exactly 1 monospace character width
                _ => {
                    // Convert codepoint to hex string for Twemoji CDN
                    let hex_code = format!("{:x}", codepoint);
                    let img = format!(
                        "<img src=\"https://cdn.jsdelivr.net/gh/twitter/twemoji@latest/assets/svg/{}.svg\" alt=\"emoji\" class=\"emoji-img\">",
                        hex_code
                    );
                    if let Ok(mut memo) = EMOJI_MEMO.lock() {
                        memo.insert(c, img.clone());
                    }
                    result.push_str(&img);
                }
            }
        } else {
            // Normal character - keep as is
            result.push(c);
        }
    }

    result
}

fn get_actual_char_width(c: char) -> f32 {
    // For characters that still need width adjustments
    match c {
        '©' => 1.0, // Copyright
        '❤' => 1.0, // Heart (without variation selector)
        // All zero-width characters
        '\u{fe0f}' => 0.0, // Variation Selector-16
        '\u{200d}' => 0.0, // Zero Width Joiner
        '\u{200b}' => 0.0, // Zero Width Space
        '\u{200c}' => 0.0, // Zero Width Non-Joiner
        '\u{200f}' => 0.0, // Right-to-Left Mark
        '\u{feff}' => 0.0, // Zero Width No-Break Space
        _ => c.width().unwrap_or(1) as f32,
    }
}

pub fn count_visual_width_excluding_html(text: &str) -> usize {
    let mut visual_width: f32 = 0.0;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '<' {
            // Check if this looks like an HTML tag (letter or / after <)
            if i + 1 < chars.len() && (chars[i + 1].is_alphabetic() || chars[i + 1] == '/') {
                // Check if it's an <img> tag (which renders and takes up space)
                let is_img_tag = i + 3 < chars.len()
                    && chars[i + 1] == 'i'
                    && chars[i + 2] == 'm'
                    && chars[i + 3] == 'g';

                // Skip the entire HTML tag until we find the closing >
                i += 1; // Skip the <
                while i < chars.len() && chars[i] != '>' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // Skip the >
                }

                // If it was an <img> tag, count it as exactly 1 character width
                // CSS forces width:1ch so this should be exact
                if is_img_tag {
                    visual_width += 1.0; // Exactly 1 character
                }
            } else {
                // Not an HTML tag, count its visual width
                visual_width += get_actual_char_width(chars[i]);
                i += 1;
            }
        } else if chars[i] == '&' {
            // Check for HTML entities like &lt; &gt; &amp; &quot; &#39;
            let remaining: String = chars[i..].iter().collect();
            if remaining.starts_with("&lt;") {
                visual_width += 1.0; // <
                i += 4;
            } else if remaining.starts_with("&gt;") {
                visual_width += 1.0; // >
                i += 4;
            } else if remaining.starts_with("&amp;") {
                visual_width += 1.0; // &
                i += 5;
            } else if remaining.starts_with("&quot;") {
                visual_width += 1.0; // "
                i += 6;
            } else if remaining.starts_with("&#39;") {
                visual_width += 1.0; // '
                i += 5;
            } else if remaining.starts_with("&nbsp;") {
                visual_width += 1.0; // non-breaking space
                i += 6;
            } else {
                // Not a recognized entity, count the &
                visual_width += get_actual_char_width(chars[i]);
                i += 1;
            }
        } else {
            // Regular character - use actual width (with overrides for problematic chars)
            visual_width += get_actual_char_width(chars[i]);
            i += 1;
        }
    }

    // Round to nearest integer for space padding
    visual_width.round() as usize
}

fn create_content_line(
    left_border: &str,
    content: &str,
    right_border: &str,
    target_width: usize,
    center: bool,
) -> String {
    // Count VISUAL WIDTH excluding HTML tags
    // (so <a href="#">TEXT</a> with emoji counts properly: TEXT = 4, 🄯 = 2, etc.)
    let content_visual_width = count_visual_width_excluding_html(content);

    if content_visual_width >= target_width {
        // Content is too long
        format!("{}{}{}\n", left_border, content, right_border)
    } else {
        // Pad with spaces to reach exact visual width
        let spaces_needed = target_width - content_visual_width;

        let line_content = if center {
            let left_spaces = spaces_needed / 2;
            let right_spaces = spaces_needed - left_spaces;
            format!(
                "{}{}{}",
                " ".repeat(left_spaces),
                content,
                " ".repeat(right_spaces)
            )
        } else {
            format!("{}{}", content, " ".repeat(spaces_needed))
        };

        format!("{}{}{}\n", left_border, line_content, right_border)
    }
}

fn create_title_lines(
    title: &str,
    left_border: &str,
    right_border: &str,
    horizontal_visual_width: usize,
) -> String {
    if count_visual_width_excluding_html(title) > horizontal_visual_width {
        let wrapped = wrap_text(title, horizontal_visual_width);
        if wrapped.is_empty() {
            create_content_line(
                left_border,
                title,
                right_border,
                horizontal_visual_width,
                true,
            )
        } else {
            wrapped
                .iter()
                .map(|line| {
                    create_content_line(
                        left_border,
                        line,
                        right_border,
                        horizontal_visual_width,
                        true,
                    )
                })
                .collect::<String>()
        }
    } else {
        create_content_line(
            left_border,
            title,
            right_border,
            horizontal_visual_width,
            true,
        )
    }
}

// New function to determine box style based on width
fn get_box_chars(
    width: usize,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    if width < 20 {
        // Minimal style for very narrow screens
        ("+", "-", "+", "|", "|")
    } else if width < 40 {
        // Simple style for narrow screens
        ("┌", "─", "┐", "│", "│")
    } else {
        // Full style for wider screens
        ("╔", "═", "╗", "║", "║")
    }
}

pub fn create_footer_box(content: &str, width: usize) -> String {
    let actual_width = width.max(10); // Minimum width of 10 characters

    // Content width = total width minus 2 (for left and right borders)
    // This ensures the horizontal line and content have the same width
    let content_width = actual_width - 2;

    let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
    let bottom_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "└"
    } else {
        "╚"
    };
    let bottom_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┘"
    } else {
        "╝"
    };

    // The horizontal line is exactly content_width characters
    let horizontal_line = horizontal.repeat(content_width);

    let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
    let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

    // Calculate the actual VISUAL WIDTH of the horizontal line and match it
    let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);
    let middle = create_content_line(
        left_border,
        content,
        right_border,
        horizontal_visual_width,
        true,
    );

    format!("{}{}{}", top, middle, bottom)
}

pub fn create_header_box(title: &str, content: &str, width: usize) -> String {
    let actual_width = width.max(10); // Minimum width of 10 characters
    let content_width = actual_width.saturating_sub(2);

    let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
    let bottom_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "└"
    } else {
        "╚"
    };
    let bottom_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┘"
    } else {
        "╝"
    };
    let header_sep_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "├"
    } else {
        "╠"
    };
    let header_sep_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┤"
    } else {
        "╣"
    };

    let horizontal_line = horizontal.repeat(content_width);
    let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
    let header_sep = format!(
        "{}{}{}\n",
        header_sep_left, horizontal_line, header_sep_right
    );
    let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

    // Calculate visual width of horizontal line
    let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);

    // Center the title
    let title_lines = create_title_lines(
        title,
        left_border,
        right_border,
        horizontal_visual_width,
    );

    // Split content into lines and pad each line with 1 space margin on each side
    let content_lines = content
        .lines()
        .map(|line| {
            let line_with_margin = format!(" {} ", line);
            create_content_line(
                left_border,
                &line_with_margin,
                right_border,
                horizontal_visual_width,
                false,
            )
        })
        .collect::<String>();

    format!(
        "{}{}{}{}{}",
        top, title_lines, header_sep, content_lines, bottom
    )
}

/// Create a post header box with title/date at top, divider, and back link at bottom
pub fn create_post_header_box(title: &str, date: &str, width: usize) -> String {
    let actual_width = width.max(10);
    let content_width = actual_width.saturating_sub(2);

    let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
    let bottom_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "└"
    } else {
        "╚"
    };
    let bottom_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┘"
    } else {
        "╝"
    };
    let sep_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "├"
    } else {
        "╠"
    };
    let sep_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┤"
    } else {
        "╣"
    };

    let horizontal_line = horizontal.repeat(content_width);
    let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);

    let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
    let sep = format!("{}{}{}\n", sep_left, horizontal_line, sep_right);
    let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

    // Title line (centered, wrapped if too long)
    let title_lines = create_title_lines(
        title,
        left_border,
        right_border,
        horizontal_visual_width,
    );

    // Date line
    let date_with_margin = format!(" {} ", date);
    let date_line = create_content_line(
        left_border,
        &date_with_margin,
        right_border,
        horizontal_visual_width,
        false,
    );

    // Empty line
    let empty_line = create_content_line(
        left_border,
        "",
        right_border,
        horizontal_visual_width,
        false,
    );

    // Back to blog link
    let back_link = " <a href=\"/blog\">&lt;&lt; Back to Blog</a> ";
    let back_line = create_content_line(
        left_border,
        back_link,
        right_border,
        horizontal_visual_width,
        false,
    );

    format!(
        "{}{}{}{}{}{}{}",
        top, title_lines, date_line, empty_line, sep, back_line, bottom
    )
}

pub fn create_about_box_with_ascii(
    title: &str,
    text_content: &str,
    ascii_art: &str,
    width: usize,
) -> String {
    let actual_width = width.max(10); // Minimum width of 10 characters
    let content_width = actual_width.saturating_sub(2);

    let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
    let bottom_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "└"
    } else {
        "╚"
    };
    let bottom_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┘"
    } else {
        "╝"
    };
    let header_sep_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "├"
    } else {
        "╠"
    };
    let header_sep_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┤"
    } else {
        "╣"
    };

    let horizontal_line = horizontal.repeat(content_width);
    let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
    let header_sep = format!(
        "{}{}{}\n",
        header_sep_left, horizontal_line, header_sep_right
    );
    let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

    // Calculate visual width of horizontal line
    let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);

    // Center the title
    let title_lines = create_title_lines(
        title,
        left_border,
        right_border,
        horizontal_visual_width,
    );

    // Create ASCII art content with proper box borders
    let ascii_art_lines: Vec<&str> = ascii_art.lines().collect();
    let ascii_art_content = ascii_art_lines
        .iter()
        .map(|line| {
            format!(
                "{}<span class=\"pfp-small\">{}</span>{}\n",
                left_border, line, right_border
            )
        })
        .collect::<String>();

    // Create text content lines with margins
    let text_content_lines = text_content
        .lines()
        .map(|line| {
            let line_with_margin = format!(" {} ", line);
            create_content_line(
                left_border,
                &line_with_margin,
                right_border,
                horizontal_visual_width,
                false,
            )
        })
        .collect::<String>();

    format!(
        "{}{}{}{}{}{}",
        top, title_lines, header_sep, ascii_art_content, text_content_lines, bottom
    )
}

pub fn create_nav_box(items: &[NavItem], width: usize, base_url: &str) -> String {
    let actual_width = width.max(10); // Minimum width of 10 characters
    let content_width = actual_width.saturating_sub(2);

    let (top_left, horizontal, top_right, left_border, right_border) = get_box_chars(actual_width);
    let bottom_left = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "└"
    } else {
        "╚"
    };
    let bottom_right = if actual_width < 20 {
        "+"
    } else if actual_width < 40 {
        "┘"
    } else {
        "╝"
    };

    let nav_items = items
        .iter()
        .map(|item| {
            let href = if item.href.starts_with('/') {
                format!("{}{}", base_url, item.href)
            } else {
                item.href.clone()
            };
            format!("<a href=\"{}\">{}</a>", href, item.text)
        })
        .collect::<Vec<_>>();

    let nav_content = nav_items.join(" | ");

    let horizontal_line = horizontal.repeat(content_width);
    let top = format!("{}{}{}\n", top_left, horizontal_line, top_right);
    let bottom = format!("{}{}{}\n", bottom_left, horizontal_line, bottom_right);

    // Calculate visual width of horizontal line
    let horizontal_visual_width = count_visual_width_excluding_html(&horizontal_line);

    // Center the navigation items
    let content_line = create_content_line(
        left_border,
        &nav_content,
        right_border,
        horizontal_visual_width,
        true,
    )
    .trim_end()
    .to_string();

    format!("{}{}\n{}", top, content_line, bottom)
}
