use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub destination: Option<PathBuf>,
}

impl Config {
    pub fn load() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("com", "example", "IngestApp") {
            let config_dir = proj_dirs.config_dir();
            let path = config_dir.join("config.json");
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&data) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(proj_dirs) = ProjectDirs::from("com", "example", "IngestApp") {
            let config_dir = proj_dirs.config_dir();
            if fs::create_dir_all(config_dir).is_ok() {
                let path = config_dir.join("config.json");
                if let Ok(data) = serde_json::to_string_pretty(self) {
                    let _ = fs::write(path, data);
                }
            }
        }
    }
}
