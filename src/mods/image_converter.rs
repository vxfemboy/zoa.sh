pub struct ImageConverter;

impl ImageConverter {
    pub fn new() -> Self {
        Self
    }

    pub fn convert_profile_image(
        &self,
        _image_path: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Read the static ASCII art from pfp.txt
        let ascii_content = std::fs::read_to_string("templates/ascii/pfp.txt")?;

        // Keep all lines but add proper spacing
        let lines: Vec<&str> = ascii_content.lines().collect();

        // Add spacing between lines to match ASCII title style
        let spaced_lines: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if i > 0 && !line.trim().is_empty() {
                    format!("\n{}", line)
                } else {
                    line.to_string()
                }
            })
            .collect();

        // Join with proper spacing
        let spaced_ascii = spaced_lines.join("");

        // The ASCII art now has proper spacing
        Ok(spaced_ascii)
    }
}

impl Default for ImageConverter {
    fn default() -> Self {
        Self::new()
    }
}
