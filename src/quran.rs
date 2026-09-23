use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Surah {
    pub number: u8,
    pub name_transliterated: String,
    pub is_meccan: bool,
    pub ayah_count: u16,
    pub ayahs: Vec<Ayah>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ayah {
    pub number: u16,
    pub arabic: String,
    pub english: String,
    pub juz: u8,
}

/// `(surah_index, ayah_index)` entries of one Juz in reading order.
/// Works over any loaded corpus, including the partial fallback.
pub fn juz_entries(surahs: &[Surah], juz: u8) -> Vec<(usize, usize)> {
    let mut entries = Vec::new();
    for (surah_index, surah) in surahs.iter().enumerate() {
        for (ayah_index, ayah) in surah.ayahs.iter().enumerate() {
            if ayah.juz == juz {
                entries.push((surah_index, ayah_index));
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Vec<Surah> {
        let ayahs = |numbers: &[u16], juz: u8| {
            numbers
                .iter()
                .map(|number| Ayah {
                    number: *number,
                    arabic: String::new(),
                    english: String::new(),
                    juz,
                })
                .collect()
        };
        vec![
            Surah {
                number: 1,
                name_transliterated: "A".to_string(),
                is_meccan: true,
                ayah_count: 2,
                ayahs: ayahs(&[1, 2], 1),
            },
            Surah {
                number: 2,
                name_transliterated: "B".to_string(),
                is_meccan: false,
                ayah_count: 2,
                ayahs: ayahs(&[1, 2], 2),
            },
        ]
    }

    #[test]
    fn juz_entries_cover_one_juz_in_order() {
        let surahs = corpus();
        assert_eq!(juz_entries(&surahs, 1), [(0, 0), (0, 1)]);
        assert_eq!(juz_entries(&surahs, 2), [(1, 0), (1, 1)]);
        assert!(juz_entries(&surahs, 30).is_empty());
    }
}
