use std::fs;
use std::path::PathBuf;

use crate::domain::models::CaffeineConfig;
use crate::domain::ports::ConfigRepository;

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"))
        .join("caffeine/config.toml")
}

pub struct FileConfigRepository;

impl ConfigRepository for FileConfigRepository {
    fn load(&self) -> CaffeineConfig {
        let path = config_path();
        let Ok(content) = fs::read_to_string(&path) else {
            return CaffeineConfig::default();
        };
        toml::from_str(&content).unwrap_or_default()
    }
}
