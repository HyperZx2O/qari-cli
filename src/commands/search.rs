use crate::collection::{self, CollectionId};
use crate::quran::Surah;
use crate::search::search_quran;
use crate::search::{search_collection, CollectionChapter};

pub fn run(query: &str, surahs: &[Surah], collection: Option<&str>) -> Result<(), String> {
    if query.trim().chars().count() < 2 {
        return Err("Search query must contain at least 2 characters".to_string());
    }

    let collection = CollectionId::from_cli(collection)?;
    if collection == CollectionId::Quran {
        return run_quran(query, surahs);
    }

    let corpus = collection::load_cached_corpus(collection)?;
    if corpus.is_empty() {
        eprintln!(
            "No downloaded {} chapters to search — open them in the TUI first.",
            collection.label()
        );
        return Ok(());
    }
    let chapters: Vec<CollectionChapter> = corpus
        .iter()
        .map(|cached| CollectionChapter {
            book_id: &cached.book_id,
            book_label: cached.book_label.clone(),
            chapter: cached.chapter,
            arabic: &cached.arabic,
            english: &cached.english,
        })
        .collect();
    // CLI prints Arabic plus English where translated, mirroring read.
    let results = search_collection(collection.key(), collection.label(), &chapters, query, true);
    println!(
        "Results for \"{}\" in {} ({} found):\n",
        query.trim(),
        collection.label(),
        results.len()
    );
    for (index, result) in results.iter().enumerate() {
        println!("{:>2}. {}", index + 1, result.display);
    }
    Ok(())
}

fn run_quran(query: &str, surahs: &[Surah]) -> Result<(), String> {
    let results = search_quran(query, surahs);
    println!(
        "Results for \"{}\" ({} found):\n",
        query.trim(),
        results.len()
    );

    for (index, result) in results.iter().enumerate() {
        println!(
            "{:>2}. [{:>3}:{}]  {} — \"{}\"",
            index + 1,
            result.surah_number,
            result.ayah_number,
            result.surah_name,
            crate::wrap::truncate_clusters(&result.english_text, 100)
        );
    }
    Ok(())
}
