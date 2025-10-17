# ASCII Web

A modern Rust-powered website that generates beautiful ASCII art borders and responsive boxes for content. Built with Actix Web, Tera templates, and WebAssembly for interactive elements.

## ✨ Features

- **Dynamic ASCII Box Generation** - Responsive boxes that adapt to screen sizes
- **Interactive WASM Cat** - WebAssembly-powered ASCII cat that follows your mouse
- **Emoji Support** - Proper Unicode and emoji handling with Twemoji fallbacks
- **Responsive Design** - Mobile-first design with multiple breakpoints
- **Custom Monospace Fonts** - Departure Mono font stack with emoji fallbacks
- **Template-Based Rendering** - Clean separation of logic and presentation
- **Modular Architecture** - Well-organized codebase with proper separation of concerns

## 🏗️ Architecture

```
src/
├── main.rs          # Actix Web server and routing
├── lib.rs           # WASM module exports
├── mods.rs          # Module exports and shared types
└── mods/
    ├── ascii_art.rs # ASCII box generation and Unicode handling
    ├── stars.rs     # Animated star generation
    ├── wasm.rs      # WebAssembly cat implementation
    ├── constants.rs # Configuration constants
    ├── data.rs      # Site content and data
    ├── responsive.rs# Responsive box creation
    └── content.rs   # Content management logic
```

## 🚀 Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)
- wasm-pack (for WASM builds)

## 📦 Setup

1. Clone the repository
2. Install wasm-pack (if not already installed):
   ```bash
   cargo install wasm-pack
   ```
3. Add your fonts to the `static/fonts` directory:
   - DepartureMono-Regular.woff2
   - DepartureMono-Regular.woff
   - DepartureMono-Regular.otf

## 🏃‍♂️ Running the Project

### Development Server
```bash
cargo run
```
The server will start at `http://localhost:8080`

### Build WASM Module
```bash
./build-wasm.sh
```
This builds the WebAssembly cat module for the frontend.

### Production Build
```bash
cargo build --release
```

## 🎨 ASCII Box Generation

The project includes sophisticated ASCII box generation with:

1. **Responsive Boxes** - Automatically adapt to screen sizes (tiny, small, medium, large)
2. **Unicode Support** - Proper handling of emojis and special characters
3. **Multiple Styles** - Different box styles based on content width
4. **Text Wrapping** - Intelligent text wrapping with visual width calculation

### Box Types
- `create_header_box()` - Boxes with titles and content
- `create_footer_box()` - Simple footer boxes
- `create_nav_box()` - Navigation boxes with links

## 🐱 Interactive Features

- **WASM Cat** - An ASCII cat that follows your mouse cursor
- **Idle Animations** - Cat performs various idle animations
- **Responsive Behavior** - Cat adapts to different screen sizes
- **Performance Optimized** - Smooth 60fps animations

## 🛠️ Customization

### Content Management
Edit `src/mods/data.rs` to modify:
- Navigation items
- Blog posts
- Categories
- Comments
- Footer text

### Styling
Modify `templates/index.html.tera` for:
- Layout changes
- CSS styling
- Responsive breakpoints

### Constants
Adjust `src/mods/constants.rs` for:
- Box widths
- Screen breakpoints
- Animation settings

## 🧪 Testing

```bash
# Run tests
cargo test

# Run with output
cargo test -- --nocapture
```

## 📚 API Endpoints

- `GET /` - Main page
- `GET /cat/{action}` - Individual cat animation frames
- `GET /static/*` - Static assets

## 🔧 Development

### Code Organization
- **Clean Architecture** - Separation of concerns with modular design
- **Error Handling** - Proper error types and handling
- **Type Safety** - Strong typing throughout the codebase
- **Performance** - Optimized for both server and client

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License.

## 🙏 Acknowledgments

- [oneko.js](https://github.com/adryd325/oneko.js) - Inspiration for the interactive cat
- [Twemoji](https://twemoji.twitter.com/) - Emoji fallback support
- [Departure Mono](https://github.com/eliheuer/departure-mono) - Beautiful monospace font