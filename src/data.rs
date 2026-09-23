use crate::quran::{Ayah, Surah};
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Shared by every network call in the app.
const USER_AGENT: &str = "qari-cli/0.1.0";

const EDITIONS: [(&str, &str); 2] = [
    ("quran-uthmani", "quran_ar.json"),
    ("en.sahih", "quran_en.json"),
];

/// Curated transliterated surah names in order. Revelation type comes from
/// the downloaded edition and the ayah count from its served ayah list, so
/// no second metadata table is needed.
const SURAH_NAMES: [&str; 114] = [
    "Al-Fatihah", "Al-Baqarah", "Ali 'Imran", "An-Nisa", "Al-Ma'idah", "Al-An'am", "Al-A'raf", "Al-Anfal",
    "At-Tawbah", "Yunus", "Hud", "Yusuf", "Ar-Ra'd", "Ibrahim", "Al-Hijr", "An-Nahl",
    "Al-Isra", "Al-Kahf", "Maryam", "Taha", "Al-Anbya", "Al-Hajj", "Al-Mu'minun", "An-Nur",
    "Al-Furqan", "Ash-Shu'ara", "An-Naml", "Al-Qasas", "Al-'Ankabut", "Ar-Rum", "Luqman", "As-Sajdah",
    "Al-Ahzab", "Saba", "Fatir", "Ya-Sin", "As-Saffat", "Sad", "Az-Zumar", "Ghafir",
    "Fussilat", "Ash-Shuraa", "Az-Zukhruf", "Ad-Dukhan", "Al-Jathiyah", "Al-Ahqaf", "Muhammad", "Al-Fath",
    "Al-Hujurat", "Qaf", "Adh-Dhariyat", "At-Tur", "An-Najm", "Al-Qamar", "Ar-Rahman", "Al-Waqi'ah",
    "Al-Hadid", "Al-Mujadila", "Al-Hashr", "Al-Mumtahanah", "As-Saf", "Al-Jumu'ah", "Al-Munafiqun", "At-Taghabun",
    "At-Talaq", "At-Tahrim", "Al-Mulk", "Al-Qalam", "Al-Haqqah", "Al-Ma'arij", "Nuh", "Al-Jinn",
    "Al-Muzzammil", "Al-Muddaththir", "Al-Qiyamah", "Al-Insan", "Al-Mursalat", "An-Naba", "An-Nazi'at", "'Abasa",
    "At-Takwir", "Al-Infitar", "Al-Mutaffifin", "Al-Inshiqaq", "Al-Buruj", "At-Tariq", "Al-A'la", "Al-Ghashiyah",
    "Al-Fajr", "Al-Balad", "Ash-Shams", "Al-Layl", "Ad-Duhaa", "Ash-Sharh", "At-Tin", "Al-'Alaq",
    "Al-Qadr", "Al-Bayyinah", "Az-Zalzalah", "Al-'Adiyat", "Al-Qari'ah", "At-Takathur", "Al-'Asr", "Al-Humazah",
    "Al-Fil", "Quraysh", "Al-Ma'un", "Al-Kawthar", "Al-Kafirun", "An-Nasr", "Al-Masad", "Al-Ikhlas",
    "Al-Falaq", "An-Nas",
];

#[derive(Debug, Deserialize)]
struct ApiResponse {
    code: u16,
    data: ApiQuran,
}

#[derive(Debug, Deserialize)]
struct ApiQuran {
    surahs: Vec<ApiSurah>,
}

#[derive(Debug, Deserialize)]
struct ApiSurah {
    number: u8,
    /// Served as `"Meccan"` / `"Medinan"`.
    #[serde(rename = "revelationType")]
    revelation_type: String,
    ayahs: Vec<ApiAyah>,
}

#[derive(Debug, Deserialize)]
struct ApiAyah {
    text: String,
    #[serde(rename = "numberInSurah")]
    number_in_surah: u16,
    juz: u8,
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("qari-cli")
}

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("qari-cli")
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    fs::create_dir_all(get_data_dir())?;
    fs::create_dir_all(get_config_dir())?;
    Ok(())
}

/// One HTTP GET for the whole app: shared user agent, per-call timeout,
/// optional query parameters, and one `{what} request failed` error shape.
pub(crate) fn http_get(
    what: &str,
    url: &str,
    query: &[(&str, &str)],
    timeout_secs: u64,
) -> Result<String, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| error.to_string())?
        .get(url)
        .query(query)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(|response| response.text())
        .map_err(|error| format!("{what} request failed: {error}"))
}

pub fn load_quran() -> Result<Vec<Surah>, Box<dyn Error>> {
    let data_dir = get_data_dir();
    let all_cached = EDITIONS
        .iter()
        .all(|(_, file_name)| data_dir.join(file_name).is_file());

    if all_cached {
        match load_from_cache(&data_dir) {
            Ok(surahs) => return Ok(surahs),
            Err(_) => remove_cache(&data_dir),
        }
    } else {
        remove_cache(&data_dir);
    }

    fetch_and_cache()
}

fn load_from_cache(data_dir: &Path) -> Result<Vec<Surah>, Box<dyn Error>> {
    let ar = parse_edition(&fs::read_to_string(data_dir.join("quran_ar.json"))?)?;
    let en = parse_edition(&fs::read_to_string(data_dir.join("quran_en.json"))?)?;
    merge_editions(ar, en)
}

fn fetch_and_cache() -> Result<Vec<Surah>, Box<dyn Error>> {
    let mut raw_editions = Vec::with_capacity(EDITIONS.len());
    for (edition, _) in EDITIONS {
        raw_editions.push(fetch_edition(edition)?);
    }

    let ar = parse_edition(&raw_editions[0])?;
    let en = parse_edition(&raw_editions[1])?;
    let surahs = merge_editions(ar, en)?;

    let data_dir = get_data_dir();
    fs::create_dir_all(&data_dir)?;
    for ((_, file_name), contents) in EDITIONS.iter().zip(raw_editions.iter()) {
        write_atomic(&data_dir.join(file_name), contents)?;
    }

    Ok(surahs)
}

fn fetch_edition(edition: &str) -> Result<String, Box<dyn Error>> {
    let url = format!("https://api.alquran.cloud/v1/quran/{edition}");
    Ok(http_get("Quran", &url, &[], 90)?)
}

fn parse_edition(contents: &str) -> Result<ApiQuran, Box<dyn Error>> {
    let response: ApiResponse = serde_json::from_str(contents)?;
    if response.code != 200 {
        return Err(format!("Quran API returned code {}", response.code).into());
    }
    Ok(response.data)
}

fn merge_editions(ar: ApiQuran, en: ApiQuran) -> Result<Vec<Surah>, Box<dyn Error>> {
    if ar.surahs.len() != 114 || en.surahs.len() != 114 {
        return Err("an edition does not contain all 114 surahs".into());
    }

    let mut merged = Vec::with_capacity(114);
    for (ar_surah, en_surah) in ar.surahs.into_iter().zip(en.surahs) {
        if ar_surah.number != en_surah.number {
            return Err(format!("edition mismatch at surah {}", ar_surah.number).into());
        }

        let name = SURAH_NAMES
            .get(ar_surah.number.saturating_sub(1) as usize)
            .ok_or_else(|| format!("unknown surah number {}", ar_surah.number))?;

        if en_surah.ayahs.len() != ar_surah.ayahs.len() {
            return Err(format!("edition ayah count mismatch in surah {}", ar_surah.number).into());
        }
        // The endpoint ships no per-surah ayah count: the served list is it.
        let ayah_count = ar_surah.ayahs.len() as u16;

        let mut ayahs = Vec::with_capacity(ar_surah.ayahs.len());
        for (arabic, english) in ar_surah.ayahs.into_iter().zip(en_surah.ayahs) {
            if arabic.number_in_surah != english.number_in_surah {
                return Err(format!(
                    "edition mismatch at {}:{}",
                    ar_surah.number, arabic.number_in_surah
                )
                .into());
            }
            ayahs.push(Ayah {
                number: arabic.number_in_surah,
                arabic: arabic.text.trim_start_matches('\u{feff}').to_string(),
                english: english.text,
                juz: arabic.juz,
            });
        }

        merged.push(Surah {
            number: ar_surah.number,
            name_transliterated: (*name).to_string(),
            is_meccan: ar_surah.revelation_type.eq_ignore_ascii_case("Meccan"),
            ayah_count,
            ayahs,
        });
    }

    let total_ayahs: usize = merged.iter().map(|surah| surah.ayahs.len()).sum();
    if total_ayahs != 6236 {
        return Err(format!("expected 6236 ayahs, found {total_ayahs}").into());
    }
    Ok(merged)
}

pub(crate) fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, contents)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)
}

fn remove_cache(data_dir: &Path) {
    for (_, file_name) in EDITIONS {
        let _ = fs::remove_file(data_dir.join(file_name));
    }
}

pub fn load_fallback() -> Vec<Surah> {
    serde_json::from_str(include_str!("../data/fallback.json"))
        .expect("bundled fallback Quran data must be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fallback_is_valid() {
        let fallback = load_fallback();
        for (number, available_ayahs) in [(1, 7), (2, 1), (55, 1), (112, 4), (113, 5), (114, 6)] {
            let surah = fallback
                .iter()
                .find(|surah| surah.number == number)
                .unwrap();
            assert_eq!(surah.ayahs.len(), available_ayahs);
        }
        let kursi = fallback
            .iter()
            .find(|surah| surah.number == 2)
            .and_then(|surah| surah.ayahs.iter().find(|ayah| ayah.number == 255))
            .expect("Al-Baqarah 255 is bundled");
        assert!(kursi.english.contains("no deity except Him"));
        assert!(!kursi.arabic.is_empty());
    }

    #[test]
    fn surah_names_cover_every_surah_in_order() {
        assert_eq!(SURAH_NAMES.len(), 114);
        assert_eq!(SURAH_NAMES[0], "Al-Fatihah");
        assert_eq!(SURAH_NAMES[113], "An-Nas");
    }
}
