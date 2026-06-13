use rand::{rng, RngExt};
use serde::Serialize;

const STAR_CHARS: &[&str] = &[
    "✦", "✧", "★", "☆", "✯", "✡", "✵", "❋", "❆", "❉", "✺", "✹", "✸", "✶", "✷", "✵", "✴", "✳", "*",
    ".", "⋆", "+", "˚",
];

#[derive(Serialize)]
pub struct Star {
    pub x: i32,
    pub y: i32,
    pub char: String,
    pub delay: f32,
    pub size: f32,
}

pub fn generate_stars(count: usize) -> Vec<Star> {
    let mut stars = Vec::with_capacity(count);
    let mut rng = rng();
    for _ in 0..count {
        let star = Star {
            x: rng.random_range(0..100), // Percentage of screen width
            y: rng.random_range(0..100), // Percentage of screen height
            char: STAR_CHARS[rng.random_range(0..STAR_CHARS.len())].to_string(),
            delay: rng.random_range(0.0..5.0), // Random animation delay up to 5 seconds
            size: rng.random_range(0.8..2.0),  // Random size multiplier
        };
        stars.push(star);
    }

    stars
}
