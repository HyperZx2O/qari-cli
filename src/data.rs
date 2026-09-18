use crate::quran::{Ayah, Surah};
use crate::surah_meta::SURAH_META;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const EDITIONS: [(&str, &str); 3] = [
    ("quran-uthmani", "quran_ar.json"),
    ("en.sahih", "quran_en.json"),
    ("bn.bengali", "quran_bn.json"),
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
    ayahs: Vec<ApiAyah>,
}

#[derive(Debug, Deserialize)]
struct ApiAyah {
    text: String,
    #[serde(rename = "numberInSurah")]
    number_in_surah: u16,
    juz: u8,
    page: u16,
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("islam-cli")
}

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("islam-cli")
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    fs::create_dir_all(get_data_dir())?;
    fs::create_dir_all(get_config_dir())?;
    Ok(())
}

pub fn load_quran(show_progress: bool) -> Result<Vec<Surah>, Box<dyn Error>> {
    let data_dir = get_data_dir();
    let all_cached = EDITIONS
        .iter()
        .all(|(_, file_name)| data_dir.join(file_name).is_file());

    if all_cached {
        match load_from_cache(&data_dir) {
            Ok(surahs) => return Ok(surahs),
            Err(error) => {
                if show_progress {
                    eprintln!("Cached Quran data is invalid ({error}); downloading it again...");
                }
                remove_cache(&data_dir);
            }
        }
    } else {
        remove_cache(&data_dir);
    }

    fetch_and_cache(show_progress)
}

fn load_from_cache(data_dir: &Path) -> Result<Vec<Surah>, Box<dyn Error>> {
    let ar = parse_edition(&fs::read_to_string(data_dir.join("quran_ar.json"))?)?;
    let en = parse_edition(&fs::read_to_string(data_dir.join("quran_en.json"))?)?;
    let bn = parse_edition(&fs::read_to_string(data_dir.join("quran_bn.json"))?)?;
    merge_editions(ar, en, bn)
}

fn fetch_and_cache(show_progress: bool) -> Result<Vec<Surah>, Box<dyn Error>> {
    if show_progress {
        eprintln!("Fetching Quran data (first run)...");
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .user_agent("islam-cli/0.1.0")
        .build()?;

    let mut raw_editions = Vec::with_capacity(EDITIONS.len());
    for (edition, _) in EDITIONS {
        if show_progress {
            eprintln!("  Downloading {edition}...");
        }
        raw_editions.push(fetch_edition(&client, edition)?);
    }

    let ar = parse_edition(&raw_editions[0])?;
    let en = parse_edition(&raw_editions[1])?;
    let bn = parse_edition(&raw_editions[2])?;
    let surahs = merge_editions(ar, en, bn)?;

    let data_dir = get_data_dir();
    fs::create_dir_all(&data_dir)?;
    for ((_, file_name), contents) in EDITIONS.iter().zip(raw_editions.iter()) {
        write_atomic(&data_dir.join(file_name), contents)?;
    }

    if show_progress {
        eprintln!("Quran data cached for offline use.");
    }
    Ok(surahs)
}

fn fetch_edition(
    client: &reqwest::blocking::Client,
    edition: &str,
) -> Result<String, Box<dyn Error>> {
    let url = format!("https://api.alquran.cloud/v1/quran/{edition}");
    Ok(client.get(url).send()?.error_for_status()?.text()?)
}

fn parse_edition(contents: &str) -> Result<ApiQuran, Box<dyn Error>> {
    let response: ApiResponse = serde_json::from_str(contents)?;
    if response.code != 200 {
        return Err(format!("Quran API returned code {}", response.code).into());
    }
    Ok(response.data)
}

fn merge_editions(ar: ApiQuran, en: ApiQuran, bn: ApiQuran) -> Result<Vec<Surah>, Box<dyn Error>> {
    if ar.surahs.len() != 114 || en.surahs.len() != 114 || bn.surahs.len() != 114 {
        return Err("an edition does not contain all 114 surahs".into());
    }

    let mut merged = Vec::with_capacity(114);
    for ((ar_surah, en_surah), bn_surah) in ar.surahs.into_iter().zip(en.surahs).zip(bn.surahs) {
        if ar_surah.number != en_surah.number || ar_surah.number != bn_surah.number {
            return Err(format!("edition mismatch at surah {}", ar_surah.number).into());
        }

        let meta = SURAH_META
            .get(ar_surah.number.saturating_sub(1) as usize)
            .ok_or_else(|| format!("unknown surah number {}", ar_surah.number))?;

        if ar_surah.ayahs.len() != meta.ayah_count as usize
            || en_surah.ayahs.len() != ar_surah.ayahs.len()
            || bn_surah.ayahs.len() != ar_surah.ayahs.len()
        {
            return Err(format!("edition ayah count mismatch in surah {}", ar_surah.number).into());
        }

        let mut ayahs = Vec::with_capacity(ar_surah.ayahs.len());
        for ((arabic, english), bengali) in ar_surah
            .ayahs
            .into_iter()
            .zip(en_surah.ayahs)
            .zip(bn_surah.ayahs)
        {
            if arabic.number_in_surah != english.number_in_surah
                || arabic.number_in_surah != bengali.number_in_surah
            {
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
                bengali: bengali.text,
                juz: arabic.juz,
                page: arabic.page,
            });
        }

        merged.push(Surah {
            number: meta.number,
            name_arabic: meta.name_arabic.to_string(),
            name_transliterated: meta.name_transliterated.to_string(),
            name_meaning: meta.name_meaning.to_string(),
            is_meccan: meta.is_meccan,
            revelation_order: meta.revelation_order,
            ayah_count: meta.ayah_count,
            ayahs,
        });
    }

    let total_ayahs: usize = merged.iter().map(|surah| surah.ayahs.len()).sum();
    if total_ayahs != 6236 {
        return Err(format!("expected 6236 ayahs, found {total_ayahs}").into());
    }
    Ok(merged)
}

fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
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
        assert!(fallback
            .iter()
            .find(|surah| surah.number == 2)
            .is_some_and(|surah| surah.ayahs.iter().any(|ayah| ayah.number == 255)));
    }
}
