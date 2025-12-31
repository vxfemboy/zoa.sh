---
title: The Art of ASCII Box Drawing
date: 2024-12-25
slug: ascii-art-guide
tags: ascii, design, tutorial
---

ASCII art isn't just decoration - it's a constraint that forces elegant solutions.

## Box Drawing Characters

Here are the Unicode box-drawing characters I use:

| Character | Name | Code |
|-----------|------|------|
| ╔ | Double top-left | U+2554 |
| ╗ | Double top-right | U+2557 |
| ╚ | Double bottom-left | U+255A |
| ╝ | Double bottom-right | U+255D |
| ║ | Double vertical | U+2551 |
| ═ | Double horizontal | U+2550 |
| ╠ | Double vertical-right | U+2560 |
| ╣ | Double vertical-left | U+2563 |

## Responsive Design Challenge

The tricky part? Making boxes look good at *any* width. My solution:

```rust
fn create_content_line(content: &str, width: usize) -> String {
    let visual_width = count_visual_width(content);
    let padding = width - visual_width;
    format!("║{}{}║", content, " ".repeat(padding))
}
```

## Tips

1. Always use monospace fonts
2. Account for Unicode character widths (some are double-width!)
3. Test on multiple browsers - emoji rendering varies wildly
