use crate::quran::Surah;
use crate::search::search_quran;

pub fn run(query: &str, surahs: &[Surah]) -> Result<(), String> {
    if query.trim().chars().count() < 2 {
        return Err("Search query must contain at least 2 characters".to_string());
    }

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
            one_line(&result.english_text)
        );
    }
    Ok(())
}

fn one_line(text: &str) -> String {
    const MAX_CHARS: usize = 100;
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= MAX_CHARS {
        return normalized;
    }
    let mut shortened: String = normalized.chars().take(MAX_CHARS - 1).collect();
    shortened.push('…');
    shortened
}
