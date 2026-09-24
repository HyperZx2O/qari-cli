//! Hadith book store (Arabic + English bulk editions).
//!
//! Each hadith edition ships as ONE bulk file of numbered hadiths, so a
//! whole book loads in two fetches and caches to disk. The canonical
//! Kutub al-Sittah, Muwatta Malik and the Nawawi 40 all use the same
//! shape; the TUI shelf and `qari hadith` read the same parsed store, with
//! an in-process memo so re-opening a book never re-parses its bulk files.

use crate::data::get_data_dir;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// One hadith book available on the shelf: ids, display label, and the
/// bulk Arabic/English edition URLs. The label doubles as the Books
/// column row and the header name.
pub struct HadithMeta {
    pub id: &'static str,
    pub label: &'static str,
    pub ar_url: &'static str,
    pub en_url: &'static str,
}

pub const HADITH_META: &[HadithMeta] = &[
    HadithMeta {
        id: "bukhari",
        label: "Sahih Bukhari",
        ar_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-bukhari.min.json",
        en_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-bukhari.min.json",
    },
    HadithMeta {
        id: "muslim",
        label: "Sahih Muslim",
        ar_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-muslim.min.json",
        en_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-muslim.min.json",
    },
    HadithMeta {
        id: "abudawud",
        label: "Abu Dawud",
        ar_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-abudawud.min.json",
        en_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-abudawud.min.json",
    },
    HadithMeta {
        id: "tirmidhi",
        label: "Tirmidhi",
        ar_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-tirmidhi.min.json",
        en_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-tirmidhi.min.json",
    },
    HadithMeta {
        id: "nasai",
        label: "Nasa'i",
        ar_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-nasai.min.json",
        en_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-nasai.min.json",
    },
    HadithMeta {
        id: "ibnmajah",
        label: "Ibn Majah",
        ar_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-ibnmajah.min.json",
        en_url:
            "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-ibnmajah.min.json",
    },
    HadithMeta {
        id: "malik",
        label: "Muwatta Malik",
        ar_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-malik.min.json",
        en_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-malik.min.json",
    },
    HadithMeta {
        id: "nawawi",
        label: "Nawawi 40",
        ar_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/ara-nawawi.min.json",
        en_url: "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-nawawi.min.json",
    },
];

/// Metadata for one book id, or None when no such hadith book exists.
pub fn meta(id: &str) -> Option<&'static HadithMeta> {
    HADITH_META.iter().find(|book| book.id == id)
}

#[derive(Debug, Clone)]
pub struct HadithEntry {
    pub number: u32,
    pub arabic: String,
    pub english: String,
}

#[derive(Debug, Deserialize)]
struct BulkEdition {
    hadiths: Vec<BulkHadith>,
}

#[derive(Debug, Deserialize)]
struct BulkHadith {
    // Some editions interleave decimal sub-references ("402.2") into the
    // integer sequence; keep the raw token and drop those below.
    hadithnumber: f64,
    text: String,
}

fn cache_path(book_id: &str, language: &str) -> PathBuf {
    get_data_dir()
        .join("hadith")
        .join(format!("{book_id}_{language}.json"))
}

fn load_bulk(
    book: &HadithMeta,
    language: &str,
    url: &str,
) -> Result<Vec<(String, String)>, String> {
    let path = cache_path(book.id, language);
    crate::data::reject_symlink_components(&path)
        .map_err(|error| format!("refusing unsafe hadith cache path: {error}"))?;
    let text = match std::fs::read_to_string(&path) {
        Ok(cached) => cached,
        Err(_) => {
            let fetched = crate::data::http_get("Hadith", url, &[], 60)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            crate::data::write_atomic(&path, &fetched).map_err(|error| error.to_string())?;
            fetched
        }
    };
    parse_bulk(&text)
}

/// Strip inline HTML the API leaves in some texts (`<br>` separators).
fn sanitize(text: &str) -> String {
    text.replace("<br/>", " ")
        .replace("<br />", " ")
        .replace("<br>", " ")
}

/// The placeholder texts fawazahmed0 puts where an English translation was
/// never supplied (each 32 chars after trimming — real translations are
/// paragraphs, never these). Matched exactly so a genuine hadith that
/// *mentions* the phrase still keeps its text.
const PLACEHOLDER_ENGLISH: [&str; 2] = [
    "Hadith Translation Not Available",
    "Hadith Translation Not available",
];

/// True for rows that carry nothing: both sides blank, or an English
/// side that is only the missing-translation placeholder.
pub(crate) fn is_empty_row(arabic: &str, english: &str) -> bool {
    arabic.trim().is_empty() || PLACEHOLDER_ENGLISH.contains(&english.trim())
}

/// `(raw number token, text)` in file order. Tokens stay strings so
/// decimal sub-references ("402.2") survive the parse.
fn parse_bulk(text: &str) -> Result<Vec<(String, String)>, String> {
    let edition: BulkEdition =
        serde_json::from_str(text).map_err(|error| format!("invalid hadith payload: {error}"))?;
    if edition.hadiths.len() > MAX_HADITHS_PER_BOOK {
        return Err(format!(
            "hadith payload exceeds {MAX_HADITHS_PER_BOOK} entries"
        ));
    }
    Ok(edition
        .hadiths
        .into_iter()
        .map(|hadith| (hadith.hadithnumber.to_string(), sanitize(&hadith.text)))
        .collect())
}

/// Join Arabic to English by raw number token, keeping only whole-number
/// hadiths with something to show (the decimal "402.2" style entries are
/// duplicative sub-references and are not browsable units; both-blank
/// rows and missing-translation placeholders are dropped so no shelf
/// carries empty units). The English rows are indexed once so joining N
/// entries costs O(N), not O(N²).
fn join_entries(arabic: Vec<(String, String)>, english: Vec<(String, String)>) -> Vec<HadithEntry> {
    let mut translated = HashMap::with_capacity(english.len());
    for (id, text) in english {
        translated.entry(id).or_insert(text);
    }
    arabic
        .into_iter()
        .filter_map(|(token, arabic)| {
            let number = token.parse::<u32>().ok()?;
            let english = translated.get(&token).cloned().unwrap_or_default();
            if is_empty_row(&arabic, &english) {
                return None;
            }
            Some(HadithEntry {
                number,
                arabic,
                english,
            })
        })
        .collect()
}

/// One whole hadith book, shared without cloning: the TUI shelf fans it
/// out into chapters, the CLI reads the same store.
const MAX_HADITHS_PER_BOOK: usize = 100_000;

pub type HadithBulk = Rc<Vec<HadithEntry>>;

/// One book's hadiths with Arabic joined to English by number. Keeps a
/// per-book memo on the caller so re-opening a book reuses the parsed
/// bulk instead of re-reading and re-parsing its two JSON files.
pub fn entry_rows(
    book_id: &str,
    memo: &mut HashMap<String, HadithBulk>,
) -> Result<HadithBulk, String> {
    if let Some(rows) = memo.get(book_id) {
        return Ok(Rc::clone(rows));
    }
    let rows = Rc::new(load_entries(book_id)?);
    memo.insert(book_id.to_string(), Rc::clone(&rows));
    Ok(rows)
}

/// One book's hadiths with Arabic joined to English by number. Cold read
/// (disk, then network): long-lived callers should prefer `entry_rows`.
pub fn load_entries(book_id: &str) -> Result<Vec<HadithEntry>, String> {
    let book = meta(book_id).ok_or_else(|| format!("Unknown hadith book: {book_id}"))?;
    Ok(join_entries(
        load_bulk(book, "ar", book.ar_url)?,
        load_bulk(book, "en", book.en_url)?,
    ))
}

/// Disk-cache-only read for search: no fetch, empty when nothing was ever
/// opened (same contract as the Alkotob chapter cache).
pub fn load_cached_entries(book_id: &str) -> Vec<HadithEntry> {
    let Some(book) = meta(book_id) else {
        return Vec::new();
    };
    let read = |language: &str| -> Vec<(String, String)> {
        let path = cache_path(book.id, language);
        if crate::data::reject_symlink_components(&path).is_err() {
            return Vec::new();
        }
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| parse_bulk(&text).ok())
            .unwrap_or_default()
    };
    join_entries(read("ar"), read("en"))
}

/// Verse rows: `(verse_id, text)`.
pub type VerseRows = Vec<(String, String)>;

/// One hadith as a single-verse chapter: `(arabic, english)` verse rows.
/// `book` must be a known hadith book id; anything else is a caller bug,
/// surfaced plainly. English degrades to empty rather than failing.
pub fn chapter(
    book: &str,
    number: u32,
    want_english: bool,
) -> Result<(VerseRows, VerseRows), String> {
    if meta(book).is_none() {
        return Err(format!("Unknown hadith book: {book}"));
    }
    let entries = load_entries(book)?;
    let entry = entries
        .get((number as usize).saturating_sub(1))
        .ok_or_else(|| format!("Hadith {number} is not available"))?;
    let id = entry.number.to_string();
    let english = if want_english && !entry.english.is_empty() {
        vec![(id.clone(), entry.english.clone())]
    } else {
        Vec::new()
    };
    Ok((vec![(id, entry.arabic.clone())], english))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "metadata": {"name": "X"},
        "hadiths": [
            {"hadithnumber": 1, "text": "A<br>B"},
            {"hadithnumber": 2, "text": "C"},
            {"hadithnumber": 402.2, "text": "sub-reference"}
        ]
    }"#;

    #[test]
    fn joins_arabic_to_english_by_number_and_strips_html() {
        let entries = join_entries(parse_bulk(SAMPLE).unwrap(), parse_bulk(SAMPLE).unwrap());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].arabic, "A B");
        assert_eq!(entries[1].number, 2);
    }

    #[test]
    fn decimal_sub_references_are_dropped() {
        let entries = join_entries(parse_bulk(SAMPLE).unwrap(), Vec::new());
        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .all(|entry| entry.number == 1 || entry.number == 2));
    }

    #[test]
    fn missing_english_degrades_to_arabic_only() {
        let entries = join_entries(parse_bulk(SAMPLE).unwrap(), Vec::new());
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| entry.english.is_empty()));
    }

    #[test]
    fn duplicate_english_tokens_keep_the_first_text() {
        // `.find` semantics preserved: the first row with a number wins.
        let arabic = vec![("1".to_string(), "A".to_string())];
        let english = vec![
            ("1".to_string(), "first".to_string()),
            ("1".to_string(), "second".to_string()),
        ];
        let entries = join_entries(arabic, english);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].english, "first");
    }

    #[test]
    fn memo_hit_returns_the_same_book_without_touching_disk() {
        let rows = Rc::new(vec![HadithEntry {
            number: 7,
            arabic: "A".to_string(),
            english: "E".to_string(),
        }]);
        let mut memo = HashMap::new();
        memo.insert("bukhari".to_string(), Rc::clone(&rows));
        let hit = entry_rows("bukhari", &mut memo).unwrap();
        assert!(Rc::ptr_eq(&hit, &rows));
    }

    #[test]
    fn memo_miss_forwards_unknown_book_errors() {
        assert!(entry_rows("nope", &mut HashMap::new()).is_err());
    }

    #[test]
    fn empty_and_placeholder_rows_never_reach_the_shelf() {
        let arabic = vec![
            ("1".to_string(), "A".to_string()),
            ("2".to_string(), String::new()),
            ("3".to_string(), "C".to_string()),
            ("4".to_string(), "D".to_string()),
        ];
        let english = vec![
            ("1".to_string(), "one".to_string()),
            ("2".to_string(), String::new()),
            (
                "3".to_string(),
                "Hadith Translation Not Available".to_string(),
            ),
            ("4".to_string(), String::new()),
        ];
        let entries = join_entries(arabic, english);
        // #1 survives; #2 (blank arabic) and #3 (placeholder) drop;
        // #4 survives on its Arabic text (blank English is OK).
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].number, 1);
        assert_eq!(entries[1].number, 4);
        assert!(is_empty_row("", ""));
        assert!(!is_empty_row("A", "one"));
    }

    #[test]
    fn rejects_malformed_payloads() {
        assert!(parse_bulk("nope").is_err());
    }
}
