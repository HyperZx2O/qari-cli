pub mod random;
pub mod read;
pub mod search;
pub mod today;

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
