//! Alkotob API (Tawrat, Zabur, Injil, and more): edition → books →
//! chapters → verses, as stable versioned JSON with no key.
//!
//! Unlike the Quran path (bulk-fetched once), chapters are fetched lazily
//! one file at a time and cached verbatim on disk — full coverage would be
//! thousands of chapters. Raw responses are cached exactly like `data.rs`
//! caches its editions, so every render after the first is offline.

use crate::data::get_data_dir;
use serde::Deserialize;
use std::path::PathBuf;

pub const API_BASE: &str = "https://alkotob.org/api/v1";

#[derive(Debug, Clone)]
pub struct EditionMeta {
    pub id: String,
    pub name: String,
    pub revelation: String,
    pub language: String,
    pub direction: String,
}

#[derive(Debug, Clone)]
pub struct BookMeta {
    pub id: String,
    pub name: String,
    pub chapter_count: u32,
}

#[derive(Debug, Clone)]
pub struct Verse {
    /// Verse identifier as served (usually numeric, kept as text).
    pub id: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub edition: String,
    pub book: String,
    pub number: u32,
    pub name: String,
    pub verses: Vec<Verse>,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct ApiEdition {
    id: String,
    name: String,
    revelation: String,
    language: String,
    #[serde(default = "ltr_default")]
    direction: String,
}

#[derive(Debug, Deserialize)]
struct ApiBook {
    id: String,
    name: String,
    #[serde(rename = "chapterCount", default)]
    chapter_count: u32,
}

#[derive(Debug, Deserialize)]
struct ApiChapter {
    #[serde(default)]
    name: Option<String>,
    verses: Vec<ApiVerse>,
}

#[derive(Debug, Deserialize)]
struct ApiVerse {
    id: String,
    content: String,
}

fn ltr_default() -> String {
    "ltr".to_string()
}

fn editions_path() -> PathBuf {
    get_data_dir().join("alkotob").join("editions.json")
}

fn books_path(edition: &str) -> PathBuf {
    get_data_dir()
        .join("alkotob")
        .join(edition)
        .join("books.json")
}

fn chapter_path(edition: &str, book: &str, chapter: u32) -> PathBuf {
    get_data_dir()
        .join("alkotob")
        .join(edition)
        .join(book)
        .join(format!("{chapter}.json"))
}

pub fn parse_editions(text: &str) -> Result<Vec<EditionMeta>, String> {
    let envelope: ApiEnvelope<Vec<ApiEdition>> =
        serde_json::from_str(text).map_err(|error| format!("invalid editions payload: {error}"))?;
    Ok(envelope
        .data
        .into_iter()
        .map(|edition| EditionMeta {
            id: edition.id,
            name: edition.name,
            revelation: edition.revelation,
            language: edition.language,
            direction: edition.direction,
        })
        .collect())
}

pub fn parse_books(text: &str) -> Result<Vec<BookMeta>, String> {
    let envelope: ApiEnvelope<Vec<ApiBook>> =
        serde_json::from_str(text).map_err(|error| format!("invalid books payload: {error}"))?;
    Ok(envelope
        .data
        .into_iter()
        .map(|book| BookMeta {
            id: book.id,
            name: book.name,
            chapter_count: book.chapter_count,
        })
        .collect())
}

pub fn parse_chapter(
    edition: &str,
    book: &str,
    number: u32,
    text: &str,
) -> Result<Chapter, String> {
    let envelope: ApiEnvelope<ApiChapter> =
        serde_json::from_str(text).map_err(|error| format!("invalid chapter payload: {error}"))?;
    Ok(Chapter {
        edition: edition.to_string(),
        book: book.to_string(),
        number,
        name: envelope.data.name.unwrap_or_default(),
        verses: envelope
            .data
            .verses
            .into_iter()
            .map(|verse| Verse {
                id: verse.id,
                content: verse.content,
            })
            .collect(),
    })
}

fn load_cached_or_fetch(path: PathBuf, url: &str) -> Result<String, String> {
    if let Ok(contents) = std::fs::read_to_string(&path) {
        return Ok(contents);
    }
    let contents = crate::data::http_get("Alkotob", url, &[], 30)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    crate::data::write_atomic(&path, &contents).map_err(|error| error.to_string())?;
    Ok(contents)
}

pub fn load_editions() -> Result<Vec<EditionMeta>, String> {
    let text = load_cached_or_fetch(editions_path(), &format!("{API_BASE}/editions.json"))?;
    parse_editions(&text)
}

pub fn load_books(edition: &str) -> Result<Vec<BookMeta>, String> {
    let text = load_cached_or_fetch(
        books_path(edition),
        &format!("{API_BASE}/editions/{edition}/books.json"),
    )?;
    parse_books(&text)
}

pub fn load_chapter(edition: &str, book: &str, chapter: u32) -> Result<Chapter, String> {
    let text = load_cached_or_fetch(
        chapter_path(edition, book, chapter),
        &format!("{API_BASE}/editions/{edition}/books/{book}/chapters/{chapter}.json"),
    )?;
    parse_chapter(edition, book, chapter, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDITIONS_SAMPLE: &str = r#"{
        "apiVersion": "1.0",
        "data": [
            {"id": "tawrat", "name": "X", "revelation": "tawrat",
             "language": "ar", "languageName": "X", "direction": "rtl",
             "bookCount": 5, "chapterCount": 187, "verseCount": 5830,
             "links": {"self": "x", "books": "y", "reader": "z"}},
            {"id": "injilen", "name": "Y", "revelation": "injil",
             "language": "en", "languageName": "Y",
             "bookCount": 5, "chapterCount": 117, "verseCount": 4722,
             "links": {"self": "x", "books": "y", "reader": "z"}}
        ],
        "links": {"self": "x", "api": "y"}, "meta": {"total": 2}
    }"#;

    const BOOKS_SAMPLE: &str = r#"{
        "apiVersion": "1.0",
        "data": [
            {"id": "gen", "name": "A",
             "links": {"self": "x", "reader": "y"}},
            {"id": "exo", "name": "B", "chapterCount": 40,
             "links": {"self": "x", "reader": "y"}}
        ],
        "links": {"self": "x", "edition": "y"}, "meta": {"total": 2}
    }"#;

    const CHAPTER_SAMPLE: &str = r#"{
        "apiVersion": "1.0",
        "data": {
            "edition": {"id": "tawrat"},
            "book": {"id": "gen"},
            "id": "1", "name": "C",
            "bismillah": "D",
            "verses": [
                {"id": "1", "content": "E", "notes": []},
                {"id": "2", "content": "F", "notes": []},
                {"id": "10", "content": "G", "notes": []}
            ]
        },
        "links": {}, "meta": {}
    }"#;

    #[test]
    fn parses_editions_with_direction_default() {
        let editions = parse_editions(EDITIONS_SAMPLE).unwrap();
        assert_eq!(editions.len(), 2);
        assert_eq!(editions[0].id, "tawrat");
        assert_eq!(editions[0].direction, "rtl");
        assert_eq!(editions[0].revelation, "tawrat");
        // direction is optional in the payload; missing means ltr.
        assert_eq!(editions[1].direction, "ltr");
    }

    #[test]
    fn parses_books_preserving_order() {
        let books = parse_books(BOOKS_SAMPLE).unwrap();
        assert_eq!(books.len(), 2);
        assert_eq!(books[0].id, "gen");
        assert_eq!(books[1].chapter_count, 40);
    }

    #[test]
    fn parses_chapter_verses_in_order() {
        let chapter = parse_chapter("tawrat", "gen", 1, CHAPTER_SAMPLE).unwrap();
        assert_eq!(chapter.edition, "tawrat");
        assert_eq!(chapter.book, "gen");
        assert_eq!(chapter.number, 1);
        assert_eq!(chapter.name, "C");
        assert_eq!(
            chapter
                .verses
                .iter()
                .map(|verse| verse.id.as_str())
                .collect::<Vec<_>>(),
            ["1", "2", "10"]
        );
        assert_eq!(chapter.verses[2].content, "G");
    }

    #[test]
    fn rejects_malformed_payloads() {
        assert!(parse_editions("nope").is_err());
        assert!(parse_books("{}").is_err());
        assert!(parse_chapter("a", "b", 1, "{}").is_err());
    }

    #[test]
    fn cache_paths_nest_by_edition_book_chapter() {
        let path = chapter_path("injil", "mat", 3);
        let text = path.to_string_lossy().into_owned();
        assert!(text.contains("alkotob"));
        assert!(text.ends_with(&format!("injil{0}mat{0}3.json", std::path::MAIN_SEPARATOR)));
    }
}
