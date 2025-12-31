---
title: Parsing Markdown with pulldown-cmark
date: 2024-12-15
slug: markdown-parsing
tags: rust, markdown, tutorial
---

This blog uses `pulldown-cmark` for Markdown parsing and `syntect` for syntax highlighting.

## The Pipeline

1. Read `.md` file from `posts/` directory
2. Parse YAML frontmatter (title, date, slug)
3. Convert Markdown to HTML
4. Apply syntax highlighting to code blocks
5. Store both HTML and plain text versions

## Code

```rust
fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);
    // ... syntax highlighting logic
}
```

## Supported Features

- **Bold** and *italic* text
- `inline code` and fenced code blocks
- [Links](https://example.com)
- Tables (as shown in other posts)
- Blockquotes
- Lists (ordered and unordered)
- ~~Strikethrough~~

## Frontmatter Format

```yaml
---
title: My Post Title
date: 2024-12-15
slug: my-post-slug
---
```
