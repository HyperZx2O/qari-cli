//! Scripture collections: the Quran plus Tawrat, Zabur and Injil served
//! by the Alkotob API.
//!
//! The Quran keeps its dedicated alquran.cloud pipeline (Uthmani text,
//! Sahih English, Juz/Page metadata). Everything else arrives as Alkotob
//! edition → book → chapter → verse payloads, cached per file on disk.

use crate::alkotob;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionId {
    Quran,
    Tawrat,
    Zabur,
    Injil,
    Tanakh,
    Greek,
    // Each canonical hadith book is its own shelf row (no generic
    // "Hadith" group: every shelf must be one level deep).
    Bukhari,
    Muslim,
    AbuDawud,
    Tirmidhi,
    Nasai,
    IbnMajah,
    Malik,
    Nawawi,
}

/// `(book_id, transliterated label)` tables. One table serves nav labels
/// and CLI resolution alike, so the two can never disagree.
const TAWART_BOOKS: &[(&str, &str)] = &[
    ("gen", "Genesis"),
    ("exo", "Exodus"),
    ("lev", "Leviticus"),
    ("num", "Numbers"),
    ("deu", "Deuteronomy"),
];

const INJIL_BOOKS: &[(&str, &str)] = &[
    ("mat", "Matthew"),
    ("mar", "Mark"),
    ("luk", "Luke"),
    ("joh", "John"),
    ("act", "Acts"),
    ("rom", "Romans"),
    ("1co", "1 Corinthians"),
    ("2co", "2 Corinthians"),
    ("gal", "Galatians"),
    ("eph", "Ephesians"),
    ("phi", "Philippians"),
    ("col", "Colossians"),
    ("1th", "1 Thessalonians"),
    ("2th", "2 Thessalonians"),
    ("1ti", "1 Timothy"),
    ("2ti", "2 Timothy"),
    ("tit", "Titus"),
    ("phm", "Philemon"),
    ("heb", "Hebrews"),
    ("jam", "James"),
    ("1pe", "1 Peter"),
    ("2pe", "2 Peter"),
    ("1jo", "1 John"),
    ("2jo", "2 John"),
    ("3jo", "3 John"),
    ("jud", "Jude"),
    ("rev", "Revelation"),
];

const TANAKH_BOOKS: &[(&str, &str)] = &[
    ("gen", "Genesis"),
    ("exod", "Exodus"),
    ("lev", "Leviticus"),
    ("num", "Numbers"),
    ("deut", "Deuteronomy"),
    ("josh", "Joshua"),
    ("judg", "Judges"),
    ("ruth", "Ruth"),
    ("1sam", "1 Samuel"),
    ("2sam", "2 Samuel"),
    ("1kgs", "1 Kings"),
    ("2kgs", "2 Kings"),
    ("1chr", "1 Chronicles"),
    ("2chr", "2 Chronicles"),
    ("ezra", "Ezra"),
    ("neh", "Nehemiah"),
    ("esth", "Esther"),
    ("job", "Job"),
    ("ps", "Psalms"),
    ("prov", "Proverbs"),
    ("eccl", "Ecclesiastes"),
    ("song", "Song of Songs"),
    ("isa", "Isaiah"),
    ("jer", "Jeremiah"),
    ("lam", "Lamentations"),
    ("ezek", "Ezekiel"),
    ("dan", "Daniel"),
    ("hos", "Hosea"),
    ("joel", "Joel"),
    ("amos", "Amos"),
    ("obad", "Obadiah"),
    ("jonah", "Jonah"),
    ("mic", "Micah"),
    ("nah", "Nahum"),
    ("hab", "Habakkuk"),
    ("zeph", "Zephaniah"),
    ("hag", "Haggai"),
    ("zech", "Zechariah"),
    ("mal", "Malachi"),
];

impl CollectionId {
    /// The single library shelf, in display order: the six revelations
    /// then the eight hadith books. Column 1 lists exactly these; panels
    /// never switch books any other way.
    pub fn library() -> [CollectionId; 14] {
        [
            Self::Quran,
            Self::Tawrat,
            Self::Zabur,
            Self::Injil,
            Self::Tanakh,
            Self::Greek,
            Self::Bukhari,
            Self::Muslim,
            Self::AbuDawud,
            Self::Tirmidhi,
            Self::Nasai,
            Self::IbnMajah,
            Self::Malik,
            Self::Nawawi,
        ]
    }

    /// Book id when this collection is a hadith book on the shelf.
    pub fn hadith_id(self) -> Option<&'static str> {
        match self {
            Self::Bukhari => Some("bukhari"),
            Self::Muslim => Some("muslim"),
            Self::AbuDawud => Some("abudawud"),
            Self::Tirmidhi => Some("tirmidhi"),
            Self::Nasai => Some("nasai"),
            Self::IbnMajah => Some("ibnmajah"),
            Self::Malik => Some("malik"),
            Self::Nawawi => Some("nawawi"),
            _ => None,
        }
    }

    pub fn is_hadith(self) -> bool {
        self.hadith_id().is_some()
    }

    pub fn key(self) -> &'static str {
        if let Some(id) = self.hadith_id() {
            return id;
        }
        match self {
            Self::Quran => "quran",
            Self::Tawrat => "tawrat",
            Self::Zabur => "zabur",
            Self::Injil => "injil",
            Self::Tanakh => "tanakh",
            Self::Greek => "greek",
            _ => unreachable!("hadith ids handled above"),
        }
    }

    pub fn label(self) -> &'static str {
        if let Some(id) = self.hadith_id() {
            return crate::hadith::meta(id).map(|book| book.label).unwrap_or(id);
        }
        match self {
            Self::Quran => "Quran",
            Self::Tawrat => "Tawrat",
            Self::Zabur => "Zabur",
            Self::Injil => "Injil",
            Self::Tanakh => "Tanakh",
            Self::Greek => "Greek NT",
            _ => unreachable!("hadith ids handled above"),
        }
    }

    pub fn from_key(value: &str) -> Self {
        Self::from_key_opt(value).unwrap_or(Self::Quran)
    }

    /// Collection for an explicit token, or None when the token names no
    /// collection (so `qari read 2:255` keeps meaning the Quran).
    pub fn from_key_opt(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "quran" => Some(Self::Quran),
            "tawrat" => Some(Self::Tawrat),
            "zabur" => Some(Self::Zabur),
            "injil" => Some(Self::Injil),
            "tanakh" => Some(Self::Tanakh),
            "greek" | "gnt" => Some(Self::Greek),
            // "hadith" keeps meaning the Nawawi 40 shelf for legacy
            // configs and `qari read "hadith nawawi 1"`.
            "hadith" | "nawawi" => Some(Self::Nawawi),
            "bukhari" => Some(Self::Bukhari),
            "muslim" => Some(Self::Muslim),
            "abudawud" | "abu dawud" => Some(Self::AbuDawud),
            "tirmidhi" => Some(Self::Tirmidhi),
            "nasai" | "nasa'i" => Some(Self::Nasai),
            "ibnmajah" | "ibn majah" => Some(Self::IbnMajah),
            "malik" => Some(Self::Malik),
            _ => None,
        }
    }

    /// Collection for a CLI argument (absent means the Quran), with the
    /// shared "unknown collection" message.
    pub fn from_cli(value: Option<&str>) -> Result<Self, String> {
        match value {
            None => Ok(Self::Quran),
            Some(name) => Self::from_key_opt(name).ok_or_else(|| {
                format!("Unknown collection: {name} (try quran, tawrat, zabur, injil)")
            }),
        }
    }

    /// Collection whose primary or translated text lives in this Alkotob
    /// edition id — the inverse of `arabic_edition`/`english_edition`, so
    /// `qari books` can label editions without a second table.
    pub fn from_edition(edition: &str) -> Option<Self> {
        Self::library().into_iter().find(|candidate| {
            candidate.arabic_edition() == Some(edition)
                || candidate.english_edition() == Some(edition)
        })
    }

    /// Alkotob edition id holding this collection's primary text.
    /// None selects the Quran's dedicated alquran.cloud pipeline; hadith
    /// books branch to their own bulk-file store before this is read.
    pub fn arabic_edition(self) -> Option<&'static str> {
        match self {
            Self::Quran => None,
            Self::Tawrat => Some("tawrat"),
            Self::Zabur => Some("zabur"),
            Self::Injil => Some("injil"),
            Self::Tanakh => Some("wlc"),
            Self::Greek => Some("gnt"),
            _ => None,
        }
    }

    /// Alkotob edition id holding an English translation, when one exists.
    pub fn english_edition(self) -> Option<&'static str> {
        match self {
            Self::Injil => Some("injilen"),
            _ => None,
        }
    }

    /// Left-panel label for the library shelf.
    pub fn section_title(self) -> &'static str {
        if self.is_hadith() {
            return " Hadith ";
        }
        match self {
            Self::Quran => " Surahs ",
            Self::Zabur => " Psalms ",
            Self::Tawrat | Self::Injil | Self::Tanakh | Self::Greek => " Chapters ",
            _ => unreachable!("hadith ids handled above"),
        }
    }

    fn book_table(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Tawrat => TAWART_BOOKS,
            Self::Injil | Self::Greek => INJIL_BOOKS,
            Self::Tanakh => TANAKH_BOOKS,
            _ => &[],
        }
    }

    /// Transliterated navigation label for an Alkotob book id. Hadith
    /// books answer with their own label for their own id; unknown ids
    /// fall back to the API's own name.
    pub fn book_label(self, book_id: &str, api_name: &str) -> String {
        if let Some(id) = self.hadith_id() {
            return crate::hadith::meta(id)
                .filter(|book| book.id == book_id)
                .map(|book| book.label.to_string())
                .unwrap_or_else(|| api_name.to_string());
        }
        self.book_table()
            .iter()
            .find(|(id, _)| *id == book_id)
            .map(|(_, label)| label.to_string())
            .unwrap_or_else(|| api_name.to_string())
    }

    /// Resolve a CLI book token to an Alkotob book id: the id itself
    /// (any case) or a single-word transliteration (`Genesis`, `Matthew`).
    /// Multi-word names (`1 Corinthians`) must use the id (`1co`); every id
    /// is listed by `qari books`.
    pub fn resolve_book_id(collection: CollectionId, token: &str) -> Option<&'static str> {
        let token = token.to_ascii_lowercase();
        if let Some(id) = collection.hadith_id() {
            return crate::hadith::meta(id)
                .filter(|book| book.id == token || book.label.to_ascii_lowercase() == token)
                .map(|book| book.id);
        }
        collection.book_table().iter().find_map(|(id, label)| {
            if *id == token || (label.to_ascii_lowercase() == token && !label.contains(' ')) {
                Some(*id)
            } else {
                None
            }
        })
    }
}

/// Display label for a book id, consulting the cached books index so
/// unknown ids still show the API's own name instead of a bare id.
pub fn book_display(collection: CollectionId, book_id: &str) -> String {
    if let Some(book) = crate::hadith::meta(book_id) {
        return book.label.to_string();
    }
    match collection.arabic_edition() {
        Some(edition) => match crate::alkotob::load_books(edition) {
            Ok(books) => books
                .iter()
                .find(|book| book.id == book_id)
                .map(|book| collection.book_label(&book.id, &book.name))
                .unwrap_or_else(|| book_id.to_string()),
            Err(_) => collection.book_label(book_id, book_id),
        },
        None => book_id.to_string(),
    }
}

/// One directly-readable unit for column 2: a surah-equivalent flattened
/// with its book prefix, so every shelf is one level deep. Quran units
/// come from the loaded surahs instead and never live here.
#[derive(Debug, Clone)]
pub struct FlatUnit {
    pub book_id: String,
    pub chapter: u32,
    pub label: String,
}

/// Flatten a shelf's books into readable units: `Genesis 1 … Deuteronomy
/// 34`, `Psalm 1 … Psalm 150`, `Hadith 1 … Hadith 42`. Counts come from
/// the cached books index (or the Hadith bulk store); no network.
pub fn flat_units(collection: CollectionId, books: &[crate::alkotob::BookMeta]) -> Vec<FlatUnit> {
    if collection == CollectionId::Quran {
        return Vec::new();
    }
    // Zabur and every hadith book ship one book whose chapters are the
    // units (`Psalm 1 … 150`, `Hadith 1 … 7563`); the rest flatten their
    // books into `Book N` rows.
    if collection == CollectionId::Zabur || collection.is_hadith() {
        let prefix = if collection.is_hadith() {
            "Hadith"
        } else {
            "Psalm"
        };
        return books
            .first()
            .map(|book| {
                (1..=book.chapter_count.max(1))
                    .map(|number| FlatUnit {
                        book_id: book.id.clone(),
                        chapter: number,
                        label: format!("{prefix} {number}"),
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
    books
        .iter()
        .flat_map(|book| {
            let label = collection.book_label(&book.id, &book.name);
            (1..=book.chapter_count.max(1)).map(move |number| FlatUnit {
                book_id: book.id.clone(),
                chapter: number,
                label: format!("{label} {number}"),
            })
        })
        .collect()
}

/// One chapter's verses in each available language: `(verse_id, text)`.
/// English is empty where the edition has no translation.
#[derive(Debug, Clone, Default)]
pub struct ChapterTexts {
    pub arabic: Vec<(String, String)>,
    pub english: Vec<(String, String)>,
}

/// One cached chapter with both languages joined, for offline search.
#[derive(Debug, Clone, Default)]
pub struct CachedChapter {
    pub book_id: String,
    pub book_label: String,
    pub chapter: u32,
    pub arabic: Vec<(String, String)>,
    pub english: Vec<(String, String)>,
}

/// `(book_id, chapter_count)` pairs for picking chapters by global ordinal
/// (random/today). The Quran has its own counters and never comes here.
/// Hadith counts come from their bulk files, not the network index.
pub fn book_chapter_counts(collection: CollectionId) -> Result<Vec<(String, u32)>, String> {
    if let Some(id) = collection.hadith_id() {
        let count = crate::hadith::load_entries(id)?.len().max(1) as u32;
        return Ok(vec![(id.to_string(), count)]);
    }
    let edition = collection
        .arabic_edition()
        .ok_or_else(|| "Quran chapters are addressed by surah, not by book".to_string())?;
    Ok(crate::alkotob::load_books(edition)?
        .into_iter()
        .map(|book| (book.id, book.chapter_count.max(1)))
        .collect())
}

/// Every chapter file present in the local disk cache, both languages
/// joined by verse id. Never touches the network: unopened chapters have
/// no rows to search.
pub fn load_cached_corpus(collection: CollectionId) -> Result<Vec<CachedChapter>, String> {
    if let Some(id) = collection.hadith_id() {
        let label = collection.label();
        return Ok(crate::hadith::load_cached_entries(id)
            .into_iter()
            .map(|entry| CachedChapter {
                book_id: id.to_string(),
                book_label: label.to_string(),
                chapter: entry.number,
                arabic: vec![(entry.number.to_string(), entry.arabic)],
                english: if entry.english.is_empty() {
                    Vec::new()
                } else {
                    vec![(entry.number.to_string(), entry.english)]
                },
            })
            .collect());
    }
    let edition = collection
        .arabic_edition()
        .ok_or_else(|| "Quran search uses the loaded corpus, not the chapter cache".to_string())?;
    let books = crate::alkotob::load_books(edition)?;
    let english_edition = collection.english_edition();
    let mut corpus = Vec::new();
    for book in &books {
        let dir = crate::data::get_data_dir()
            .join("alkotob")
            .join(edition)
            .join(&book.id);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut numbers: Vec<u32> = entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry
                    .path()
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.parse().ok())
            })
            .collect();
        numbers.sort_unstable();
        for chapter in numbers {
            let Ok(arabic) = crate::alkotob::load_chapter(edition, &book.id, chapter) else {
                continue;
            };
            let english = english_edition
                .and_then(|english_edition| {
                    crate::alkotob::load_chapter(english_edition, &book.id, chapter).ok()
                })
                .map(|translated| {
                    translated
                        .verses
                        .into_iter()
                        .map(|verse| (verse.id, verse.content))
                        .collect()
                })
                .unwrap_or_default();
            corpus.push(CachedChapter {
                book_id: book.id.clone(),
                book_label: collection.book_label(&book.id, &book.name),
                chapter,
                arabic: arabic
                    .verses
                    .into_iter()
                    .map(|verse| (verse.id, verse.content))
                    .collect(),
                english,
            });
        }
    }
    Ok(corpus)
}

/// Fetch a chapter's Arabic text plus its English translation when the
/// collection has one. A missing English chapter degrades to Arabic-only
/// rather than failing the whole read.
pub fn load_chapter_texts(
    collection: CollectionId,
    book: &str,
    chapter: u32,
    want_english: bool,
) -> Result<ChapterTexts, String> {
    if collection.is_hadith() {
        let (arabic, english) = crate::hadith::chapter(book, chapter, want_english)?;
        return Ok(ChapterTexts { arabic, english });
    }
    let arabic_edition = collection
        .arabic_edition()
        .ok_or_else(|| "Quran chapters load through the Quran pipeline".to_string())?;
    let arabic = alkotob::load_chapter(arabic_edition, book, chapter)?
        .verses
        .into_iter()
        .map(|verse| (verse.id, verse.content))
        .collect();
    let mut english = Vec::new();
    if want_english {
        if let Some(english_edition) = collection.english_edition() {
            if let Ok(translated) = alkotob::load_chapter(english_edition, book, chapter) {
                english = translated
                    .verses
                    .into_iter()
                    .map(|verse| (verse.id, verse.content))
                    .collect();
            }
        }
    }
    Ok(ChapterTexts { arabic, english })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_keys_round_trip() {
        for collection in [
            CollectionId::Quran,
            CollectionId::Tawrat,
            CollectionId::Zabur,
            CollectionId::Injil,
            CollectionId::Tanakh,
            CollectionId::Greek,
            CollectionId::Bukhari,
            CollectionId::Muslim,
            CollectionId::AbuDawud,
            CollectionId::Tirmidhi,
            CollectionId::Nasai,
            CollectionId::IbnMajah,
            CollectionId::Malik,
            CollectionId::Nawawi,
        ] {
            assert_eq!(CollectionId::from_key(collection.key()), collection);
        }
        assert_eq!(CollectionId::from_key("nonsense"), CollectionId::Quran);
    }

    #[test]
    fn library_lists_every_book_once() {
        let mut seen = CollectionId::library().to_vec();
        assert_eq!(seen.len(), 14);
        // No duplicates: every shelf appears exactly once.
        let mut unique: Vec<CollectionId> = seen.clone();
        unique.sort_by_key(|id| id.key());
        seen.sort_by_key(|id| id.key());
        unique.dedup();
        assert_eq!(seen, unique);
        assert_eq!(CollectionId::library()[0], CollectionId::Quran);
        assert!(CollectionId::library()
            .iter()
            .all(|id| id.is_hadith() == (id.hadith_id().is_some())));
    }

    #[test]
    fn book_labels_cover_known_ids() {
        assert_eq!(
            CollectionId::Tawrat.book_label("gen", "X"),
            "Genesis".to_string()
        );
        assert_eq!(
            CollectionId::Injil.book_label("rev", "X"),
            "Revelation".to_string()
        );
        assert_eq!(
            CollectionId::Injil.book_label("mat", "X"),
            "Matthew".to_string()
        );
        assert_eq!(
            CollectionId::Tawrat.book_label("unknown", "الغريب"),
            "الغريب".to_string()
        );
    }

    #[test]
    fn english_exists_only_where_it_does() {
        assert_eq!(CollectionId::Injil.english_edition(), Some("injilen"));
        assert_eq!(CollectionId::Tawrat.english_edition(), None);
        assert_eq!(CollectionId::Zabur.english_edition(), None);
        assert_eq!(CollectionId::Quran.arabic_edition(), None);
    }

    #[test]
    fn resolve_book_accepts_ids_and_single_word_names() {
        use CollectionId::{Greek, Injil, Tanakh, Tawrat, Zabur};
        let resolve = CollectionId::resolve_book_id;
        assert_eq!(resolve(Tawrat, "GEN"), Some("gen"));
        assert_eq!(resolve(Tawrat, "Genesis"), Some("gen"));
        assert_eq!(resolve(Injil, "matthew"), Some("mat"));
        assert_eq!(resolve(Injil, "1co"), Some("1co"));
        assert_eq!(resolve(Injil, "1 Corinthians"), None);
        assert_eq!(resolve(Injil, "gen"), None);
        assert_eq!(resolve(Zabur, "psa"), None);
        assert_eq!(resolve(Tawrat, "mat"), None);
        assert_eq!(resolve(Tanakh, "genesis"), Some("gen"));
        assert_eq!(resolve(Tanakh, "1 Samuel"), None);
        assert_eq!(resolve(Tanakh, "1sam"), Some("1sam"));
        assert_eq!(resolve(Greek, "matthew"), Some("mat"));
        assert_eq!(resolve(Greek, "rev"), Some("rev"));
        assert_eq!(resolve(Greek, "gen"), None);
        assert_eq!(resolve(CollectionId::Nawawi, "nawawi"), Some("nawawi"));
        assert_eq!(resolve(CollectionId::Bukhari, "bukhari"), Some("bukhari"));
        assert_eq!(
            resolve(CollectionId::Bukhari, "Sahih Bukhari"),
            Some("bukhari")
        );
        assert_eq!(resolve(CollectionId::Nawawi, "gen"), None);
    }
}
