use crate::quran::Surah;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub surah_number: u8,
    pub ayah_number: u16,
    pub surah_name: String,
    pub english_text: String,
    /// Collection key (`"quran"` for Quran hits).
    pub collection: String,
    pub book_id: String,
    pub chapter: u32,
    pub verse_id: String,
    /// Preformatted overlay row.
    pub display: String,
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
        .map(|(surah, ayah, _score)| SearchResult {
            surah_number: surah.number,
            ayah_number: ayah.number,
            surah_name: surah.name_transliterated.clone(),
            english_text: ayah.english.clone(),
            collection: "quran".to_string(),
            book_id: String::new(),
            chapter: 0,
            verse_id: String::new(),
            display: format!(
                "{}:{}  {} — {}",
                surah.number, ayah.number, surah.name_transliterated, ayah.english
            ),
        })
        .collect()
}

/// One cached chapter available for searching: borrowed verse texts.
pub struct CollectionChapter<'a> {
    pub book_id: &'a str,
    pub book_label: String,
    pub chapter: u32,
    pub arabic: &'a [(String, String)],
    pub english: &'a [(String, String)],
}

/// Fuzzy search over locally cached Alkotob chapters (never the network:
/// unopened chapters simply have no rows to match). Verse-level hits;
/// selecting one opens its chapter. Empty for queries under 2 characters.
pub fn search_collection(
    collection: &str,
    collection_label: &str,
    chapters: &[CollectionChapter<'_>],
    query: &str,
    english: bool,
) -> Vec<SearchResult> {
    let query = query.trim();
    if query.chars().count() < 2 {
        return Vec::new();
    }

    let matcher = SkimMatcherV2::default().ignore_case();
    let mut candidates: Vec<(i64, SearchResult)> = Vec::new();
    for chapter in chapters {
        for (verse_id, arabic) in chapter.arabic {
            let arabic_score = matcher.fuzzy_match(arabic, query).unwrap_or(0);
            let (english_score, snippet) = if english {
                match chapter.english.iter().find(|(id, _)| id == verse_id) {
                    Some((_, text)) => {
                        (matcher.fuzzy_match(text, query).unwrap_or(0), text.clone())
                    }
                    None => (0, arabic.clone()),
                }
            } else {
                (0, arabic.clone())
            };
            let (score, snippet) = if english_score > arabic_score {
                (english_score, snippet)
            } else {
                (arabic_score, arabic.clone())
            };
            if score > 0 {
                candidates.push((
                    score,
                    SearchResult {
                        surah_number: 0,
                        ayah_number: 0,
                        surah_name: chapter.book_label.clone(),
                        english_text: snippet.clone(),
                        collection: collection.to_string(),
                        book_id: chapter.book_id.to_string(),
                        chapter: chapter.chapter,
                        verse_id: verse_id.clone(),
                        display: format!(
                            "{collection_label} · {} {}:{} — {snippet}",
                            chapter.book_label, chapter.chapter, verse_id
                        ),
                    },
                ));
            }
        }
    }

    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
    candidates
        .into_iter()
        .take(15)
        .map(|(_, result)| result)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    type Verses = Vec<(String, String)>;

    fn chapter_fixture() -> (Verses, Verses) {
        let arabic = ["نُور", "ظَلَام", "سَمَاء"]
            .into_iter()
            .enumerate()
            .map(|(index, word)| ((index + 1).to_string(), word.to_string()))
            .collect();
        let english = ["light", "darkness", "sky"]
            .into_iter()
            .enumerate()
            .map(|(index, word)| ((index + 1).to_string(), word.to_string()))
            .collect();
        (arabic, english)
    }

    fn corpus<'a>(
        arabic: &'a [(String, String)],
        english: &'a [(String, String)],
    ) -> Vec<CollectionChapter<'a>> {
        vec![CollectionChapter {
            book_id: "gen",
            book_label: "Genesis".to_string(),
            chapter: 1,
            arabic,
            english,
        }]
    }

    #[test]
    fn collection_search_matches_arabic() {
        let (arabic, english) = chapter_fixture();
        let chapters = corpus(&arabic, &english);
        let results = search_collection("tawrat", "Tawrat", &chapters, "نُور", false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verse_id, "1");
        assert!(results[0].display.contains("Genesis 1:1"));
    }

    #[test]
    fn collection_search_matches_english_only_in_english_mode() {
        let (arabic, english) = chapter_fixture();
        let chapters = corpus(&arabic, &english);
        assert_eq!(
            search_collection("tawrat", "Tawrat", &chapters, "darkness", false).len(),
            0
        );
        let results = search_collection("tawrat", "Tawrat", &chapters, "darkness", true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verse_id, "2");
    }

    #[test]
    fn collection_search_rejects_short_queries_and_caps_at_15() {
        let (arabic, english) = chapter_fixture();
        let chapters = corpus(&arabic, &english);
        assert!(search_collection("tawrat", "Tawrat", &chapters, "x", true).is_empty());
        let big_arabic: Vec<(String, String)> = (1..=30)
            .map(|index| (index.to_string(), "نُور likelihood".to_string()))
            .collect();
        let big = vec![CollectionChapter {
            book_id: "gen",
            book_label: "Genesis".to_string(),
            chapter: 1,
            arabic: &big_arabic,
            english: &[],
        }];
        assert_eq!(
            search_collection("tawrat", "Tawrat", &big, "likelihood", false).len(),
            15
        );
    }

    #[test]
    fn quran_search_caps_at_15_and_boosts_surah_names() {
        let surahs = crate::data::load_fallback();
        assert!(search_quran("mercy", &surahs).len() <= 15);
        assert!(search_quran("rahman", &surahs)
            .iter()
            .any(|result| result.surah_number == 55));
    }
}
