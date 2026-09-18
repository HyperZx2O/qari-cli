#![allow(dead_code)]

#[path = "../src/surah_meta.rs"]
mod surah_meta;

use surah_meta::SURAH_META;

#[test]
fn test_all_114_surahs_present() {
    assert_eq!(SURAH_META.len(), 114);
}

#[test]
fn test_surah_numbers_sequential() {
    for (index, meta) in SURAH_META.iter().enumerate() {
        assert_eq!(meta.number as usize, index + 1);
    }
}

#[test]
fn test_ayah_counts_correct() {
    assert_eq!(SURAH_META[0].ayah_count, 7);
    assert_eq!(SURAH_META[1].ayah_count, 286);
    assert_eq!(SURAH_META[113].ayah_count, 6);
    assert_eq!(
        SURAH_META
            .iter()
            .map(|meta| meta.ayah_count as usize)
            .sum::<usize>(),
        6236
    );
}
