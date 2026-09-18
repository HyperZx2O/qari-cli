use crate::data::get_config_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub last_surah: u8,
    pub last_ayah: u16,
    pub language: String,
    pub theme: String,
    pub latitude: f64,
    pub longitude: f64,
    pub calc_method: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_surah: 1,
            last_ayah: 1,
            language: "en".to_string(),
            theme: "dark".to_string(),
            latitude: 23.8103,
            longitude: 90.4125,
            calc_method: 0,
        }
    }
}

pub fn load_config() -> Config {
    let path = get_config_dir().join("config.toml");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &Config) {
    let dir = get_config_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(content) = toml::to_string_pretty(config) {
        let _ = std::fs::write(dir.join("config.toml"), content);
    }
}
