use std::fs;
use std::path::Path;
use tera::Tera;

use ascii_web::mods::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Generating static site...");

    // Create output directory
    let output_dir = "dist";
    if Path::new(output_dir).exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir)?;

    // Initialize Tera templates
    let tera = Tera::new("templates/**/*")?;

    // Create content manager and page context
    let content_manager = ContentManager::new();
    let context = content_manager.create_page_context()?;

    // Build template context
    let template_context = TemplateContextBuilder::new()
        .with_page_context(&context)
        .build();

    // Render main page
    let rendered = tera.render("index.html.tera", &template_context)?;
    fs::write(format!("{}/index.html", output_dir), rendered)?;

    // Copy static files
    copy_dir_recursive("static", &format!("{}/static", output_dir))?;

    // Generate individual cat animation pages
    let cat_dir = "templates/ascii/cat";
    if Path::new(cat_dir).exists() {
        fs::create_dir_all(format!("{}/cat", output_dir))?;

        for entry in fs::read_dir(cat_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)?;
                    fs::write(format!("{}/cat/{}.txt", output_dir, filename), content)?;
                }
            }
        }
    }

    println!(
        "Static site generated successfully in '{}' directory!",
        output_dir
    );
    Ok(())
}

fn copy_dir_recursive(src: &str, dst: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = Path::new(dst).join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(src_path.to_str().unwrap(), dst_path.to_str().unwrap())?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
