# ASCII Web

A Rust-powered website that generates beautiful ASCII art borders and boxes for content. Built with Rocket and Tera templates.

## Features

- Dynamic ASCII box generation
- Responsive design
- Custom monospace font support
- Template-based rendering

## Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)

## Setup

1. Clone the repository
2. Add your fonts to the `static/fonts` directory:
   - DepartureMono-Regular.woff2
   - DepartureMono-Regular.woff
   - DepartureMono-Regular.otf

## Running the Project

```bash
cargo run
```

The server will start at `http://localhost:8000`

## Project Structure

- `src/main.rs` - Main application code and ASCII box generation
- `templates/` - Tera templates
- `static/` - Static assets (fonts, etc.)

## ASCII Box Generation

The project includes two main box generation functions:

1. `create_box()` - Creates a simple ASCII box
2. `create_header_box()` - Creates a box with a header section

## Customization

You can modify the templates in `templates/index.html.tera` to change the layout and content of the site. 


# notes:
https://github.com/adryd325/oneko.js - for cat following mouse