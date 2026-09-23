//! Ayah and chapter bookmarks persisted as a JSON list of string keys:
//! `quran:2:255` for Quran ayahs, `tawrat:gen:1:1` for verses, and
//! `bukhari:bukhari:402` for whole single-verse hadith chapters.

use crate::data::get_data_dir;
use std::path::{Path, PathBuf};

fn bookmarks_path() -> PathBuf {
    get_data_dir().join("bookmarks.json")
}

pub fn quran_key(surah: u8, ayah: u16) -> String {
    format!("quran:{surah}:{ayah}")
}

pub fn chapter_key(collection: &str, book: &str, chapter: u32) -> String {
    format!("{collection}:{book}:{chapter}")
}

pub fn verse_key(collection: &str, book: &str, chapter: u32, verse: &str) -> String {
    format!("{collection}:{book}:{chapter}:{verse}")
}

pub fn load() -> Result<Vec<String>, String> {
    load_from(&bookmarks_path())
}

pub fn load_from(path: &Path) -> Result<Vec<String>, String> {
    let contents = match std::fs::read_to_string(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
        Ok(contents) => contents,
    };
    serde_json::from_str(&contents).map_err(|error| format!("bookmarks file is invalid: {error}"))
}

pub fn save(bookmarks: &[String]) -> Result<(), String> {
    save_to(&bookmarks_path(), bookmarks)
}

pub fn save_to(path: &Path, bookmarks: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let contents = serde_json::to_string_pretty(bookmarks).map_err(|error| error.to_string())?;
    std::fs::write(path, contents).map_err(|error| error.to_string())
}

pub fn is_bookmarked(key: &str) -> bool {
    load().is_ok_and(|bookmarks| bookmarks.iter().any(|marked| marked == key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("qari-test-{}-{name}.json", std::process::id()))
    }

    #[test]
    fn bookmark_toggle_round_trip() {
        let path = temp_path("toggle");
        let _ = std::fs::remove_file(&path);
        assert_eq!(load_from(&path).unwrap(), Vec::<String>::new());

        let mut bookmarks = load_from(&path).unwrap();
        bookmarks.push(quran_key(2, 255));
        bookmarks.push(chapter_key("tawrat", "gen", 1));
        save_to(&path, &bookmarks).unwrap();
        let reloaded = load_from(&path).unwrap();
        assert!(reloaded.contains(&"quran:2:255".to_string()));
        assert!(reloaded.contains(&"tawrat:gen:1".to_string()));

        let mut bookmarks = load_from(&path).unwrap();
        bookmarks.retain(|bookmark| bookmark != "quran:2:255");
        save_to(&path, &bookmarks).unwrap();
        assert!(!load_from(&path)
            .unwrap()
            .contains(&"quran:2:255".to_string()));
        let _ = std::fs::remove_file(&path);
    }
}
