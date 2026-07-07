#![allow(dead_code)]
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct DevConfig {
    #[serde(default)]
    pub mock_data: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct SprawlConfig {
    #[serde(default)]
    pub dev: DevConfig,
}

impl SprawlConfig {
    pub fn load_from_dir(dir: &Path) -> Self {
        let config_path = dir.join(".sprawl.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }
}
