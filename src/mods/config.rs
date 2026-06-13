use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub paths: PathConfig,
    pub content: ContentConfig,
    pub debug: bool,
    pub cache_enabled: bool,
    pub cache_capacity: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PathConfig {
    pub templates: PathBuf,
    pub static_files: PathBuf,
    pub cat_animations: PathBuf,
    pub title_art: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentConfig {
    pub star_count: usize,
    pub box_widths: BoxWidths,
    pub divider_lengths: DividerLengths,
    pub wrap_widths: WrapWidths,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoxWidths {
    pub tiny: usize,
    pub small: usize,
    pub medium: usize,
    pub large: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DividerLengths {
    pub tiny: usize,
    pub small: usize,
    pub medium: usize,
    pub large: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WrapWidths {
    pub tiny: usize,
    pub small: usize,
    pub medium: usize,
    pub large: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: None,
            },
            paths: PathConfig {
                templates: PathBuf::from("templates"),
                static_files: PathBuf::from("static"),
                cat_animations: PathBuf::from("templates/ascii/cat"),
                title_art: PathBuf::from("templates/ascii/title.txt"),
            },
            content: ContentConfig {
                star_count: 150,
                box_widths: BoxWidths {
                    tiny: 30,
                    small: 40,
                    medium: 60,
                    large: 80,
                },
                divider_lengths: DividerLengths {
                    tiny: 24,
                    small: 34,
                    medium: 54,
                    large: 74,
                },
                wrap_widths: WrapWidths {
                    tiny: 22,
                    small: 32,
                    medium: 52,
                    large: 72,
                },
            },
            debug: false,
            cache_enabled: false,
            cache_capacity: 256,
        }
    }
}

impl Config {
    pub fn load() -> crate::mods::Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::Environment::with_prefix("ZOA"));

        // Set defaults manually since set_defaults doesn't exist
        let default_config = Config::default();

        // Try to build and deserialize
        match settings.build() {
            Ok(config) => config
                .try_deserialize()
                .map_err(crate::mods::AppError::Config),
            Err(_) => {
                // If config file doesn't exist, return defaults
                Ok(default_config)
            }
        }
    }
}
