#![allow(dead_code)]

#[path = "../src/data.rs"]
mod data;
#[path = "../src/quran.rs"]
mod quran;
#[path = "../src/surah_meta.rs"]
mod surah_meta;

#[test]
fn test_read_ayat_al_kursi() {
    let surahs = data::load_fallback();
    let surah2 = surahs.iter().find(|surah| surah.number == 2).unwrap();
    let ayah255 = surah2.ayahs.iter().find(|ayah| ayah.number == 255).unwrap();
    assert!(ayah255.english.contains("no deity except Him"));
    assert!(!ayah255.arabic.is_empty());
}
