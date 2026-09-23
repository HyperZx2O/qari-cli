pub mod books;
pub mod hadith;
pub mod pray;
pub mod random;
pub mod read;
pub mod search;
pub mod today;

use crate::collection::{self, CollectionId};
use crate::quran::{Ayah, Surah};

pub fn ayah_at_index(surahs: &[Surah], mut index: usize) -> Option<(&Surah, &Ayah)> {
    for surah in surahs {
        if index < surah.ayahs.len() {
            return Some((surah, &surah.ayahs[index]));
        }
        index = index.saturating_sub(surah.ayahs.len());
    }
    None
}

pub fn total_ayahs(surahs: &[Surah]) -> usize {
    surahs.iter().map(|surah| surah.ayahs.len()).sum()
}

/// Pick `(book_id, chapter, optional verse)` by global ordinal across a
/// collection's chapters. The verse is `Some` only for Zabur, whose reads
/// are single verses; Tawrat and Injil read whole chapters.
pub fn chapter_at_ordinal(
    collection: CollectionId,
    seed: usize,
) -> Result<(String, u32, Option<String>), String> {
    let books = collection::book_chapter_counts(collection)?;
    let total: usize = books.iter().map(|(_, count)| *count as usize).sum();
    if total == 0 {
        return Err("No chapters are available".to_string());
    }
    let mut index = seed % total;
    for (book_id, count) in &books {
        let count = *count as usize;
        if index < count {
            let chapter = (index + 1) as u32;
            if collection == CollectionId::Zabur {
                let texts = collection::load_chapter_texts(collection, book_id, chapter, false)?;
                if texts.arabic.is_empty() {
                    return Err(format!("No verses found in {book_id} {chapter}"));
                }
                let verse = texts.arabic[seed % texts.arabic.len()].0.clone();
                return Ok((book_id.clone(), chapter, Some(verse)));
            }
            return Ok((book_id.clone(), chapter, None));
        }
        index -= count;
    }
    Err("No chapters are available".to_string())
}
