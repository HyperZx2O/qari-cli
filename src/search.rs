use crate::quran::Surah;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub surah_number: u8,
    pub ayah_number: u16,
    pub surah_name: String,
    pub english_text: String,
    pub score: i64,
}

pub fn search_quran(query: &str, surahs: &[Surah]) -> Vec<SearchResult> {
    let query = query.trim().to_lowercase();
    if query.chars().count() < 2 {
        return Vec::new();
    }

    let matcher = SkimMatcherV2::default();
    let mut results = Vec::new();

    for surah in surahs {
        let name_score = matcher
            .fuzzy_match(&surah.name_transliterated.to_lowercase(), &query)
            .map_or(0, |score| score + 500);

        for ayah in &surah.ayahs {
            let text_score = matcher
                .fuzzy_match(&ayah.english.to_lowercase(), &query)
                .unwrap_or(0);
            let score = name_score + text_score;

            if score > 0 {
                results.push(SearchResult {
                    surah_number: surah.number,
                    ayah_number: ayah.number,
                    surah_name: surah.name_transliterated.clone(),
                    english_text: ayah.english.clone(),
                    score,
                });
            }
        }
    }

    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.surah_number.cmp(&right.surah_number))
            .then_with(|| left.ayah_number.cmp(&right.ayah_number))
    });
    results.truncate(15);
    results
}
