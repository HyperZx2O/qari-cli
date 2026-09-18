use crate::quran::Surah;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub surah_number: u8,
    pub ayah_number: u16,
    pub surah_name: String,
    pub english_text: String,
    #[allow(dead_code)]
    pub score: i64,
}

pub fn search_quran(query: &str, surahs: &[Surah]) -> Vec<SearchResult> {
    let query = query.trim();
    if query.chars().count() < 2 {
        return Vec::new();
    }

    let matcher = SkimMatcherV2::default().ignore_case();
    let mut candidates = Vec::new();

    for surah in surahs {
        let name_score = matcher
            .fuzzy_match(&surah.name_transliterated, query)
            .map_or(0, |score| score + 500);

        for ayah in &surah.ayahs {
            let text_score = matcher.fuzzy_match(&ayah.english, query).unwrap_or(0);
            let score = name_score + text_score;

            if score > 0 {
                candidates.push((surah, ayah, score));
            }
        }
    }

    candidates.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| left.0.number.cmp(&right.0.number))
            .then_with(|| left.1.number.cmp(&right.1.number))
    });
    candidates
        .into_iter()
        .take(15)
        .map(|(surah, ayah, score)| SearchResult {
            surah_number: surah.number,
            ayah_number: ayah.number,
            surah_name: surah.name_transliterated.clone(),
            english_text: ayah.english.clone(),
            score,
        })
        .collect()
}
