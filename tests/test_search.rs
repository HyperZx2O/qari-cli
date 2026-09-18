#![allow(dead_code)]

#[path = "../src/data.rs"]
mod data;
#[path = "../src/quran.rs"]
mod quran;
#[path = "../src/search.rs"]
mod search;
#[path = "../src/surah_meta.rs"]
mod surah_meta;

#[test]
fn test_search_returns_max_15() {
    let results = search::search_quran("mercy", &data::load_fallback());
    assert!(results.len() <= 15);
}

#[test]
fn test_search_surah_name_boost() {
    let results = search::search_quran("rahman", &data::load_fallback());
    assert!(results.iter().any(|result| result.surah_number == 55));
}
