use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub paths: PathConfig,
    pub content: ContentConfig,
    #[serde(default)]
    pub pfp: PfpConfig,
    pub debug: bool,
    pub cache_enabled: bool,
    pub cache_capacity: usize,
}

/// `/about` profile-portrait (ANSI half-block) settings.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PfpConfig {
    /// Fetch `url` and regenerate the portrait on server startup. If the fetch
    /// fails (offline/CI), the committed `templates/ascii/avatar.*` is kept.
    pub auto_update: bool,
    /// Source image — an http(s) URL (e.g. the GitHub avatar) or a local path.
    pub url: String,
    /// Portrait width in half-block cells (box width = `width + 4`).
    pub width: usize,
    /// Brightness added after contrast, in [-1.0, 1.0]. 0.0 = none.
    pub brightness: f64,
    /// Contrast multiplier around mid-gray. 1.0 = none, >1 punchier.
    pub contrast: f64,
    /// Skip the startup refresh if the committed portrait was regenerated within
    /// this many seconds. Avoids refetching on every restart / dev reload (which
    /// would otherwise loop with a file watcher). 0 = always refresh.
    pub refresh_interval_secs: u64,
}

impl Default for PfpConfig {
    fn default() -> Self {
        Self {
            auto_update: false,
            url: "https://github.com/vxfemboy.png".to_string(),
            width: 51,
            brightness: 0.0,
            contrast: 1.0,
            refresh_interval_secs: 21600, // 6h
        }
    }
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
            pfp: PfpConfig::default(),
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
