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
    pub collection: String,
    pub last_book: String,
    pub last_chapter: u32,
    pub intro_shown: bool,
    pub reduced_motion: bool,
    pub rtl_mode: String,
    pub transliteration: bool,
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
            collection: "quran".to_string(),
            last_book: String::new(),
            last_chapter: 1,
            intro_shown: false,
            reduced_motion: false,
            rtl_mode: "auto".to_string(),
            transliteration: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_config_defaults_intro_to_not_shown() {
        let config: Config = toml::from_str(
            r#"
last_surah = 2
last_ayah = 255
language = "en"
theme = "dark"
latitude = 23.8103
longitude = 90.4125
"#,
        )
        .unwrap();
        assert!(!config.intro_shown);
        assert_eq!(config.last_surah, 2);
    }
}
