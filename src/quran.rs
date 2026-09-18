use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Surah {
    pub number: u8,
    pub name_arabic: String,
    pub name_transliterated: String,
    pub name_meaning: String,
    pub is_meccan: bool,
    pub revelation_order: u8,
    pub ayah_count: u16,
    pub ayahs: Vec<Ayah>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ayah {
    pub number: u16,
    pub arabic: String,
    pub english: String,
    pub bengali: String,
    pub juz: u8,
    pub page: u16,
}
