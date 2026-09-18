use crate::data::get_data_dir;
use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Hadith {
    hadithnumber: u16,
    text: String,
    reference: HadithReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HadithReference {
    book: u16,
    hadith: u16,
}

#[derive(Deserialize)]
struct HadithResponse {
    hadiths: Vec<Hadith>,
}

#[derive(Serialize, Deserialize)]
struct CachedHadith {
    date: String,
    hadith: Hadith,
}

pub fn run() -> Result<(), String> {
    let today = Local::now();
    let date = today.format("%Y-%m-%d").to_string();
    let cache_path = get_data_dir().join("hadith_cache.json");

    let hadith = fs::read_to_string(&cache_path)
        .ok()
        .and_then(|contents| serde_json::from_str::<CachedHadith>(&contents).ok())
        .filter(|cached| cached.date == date)
        .map(|cached| cached.hadith)
        .map(Ok)
        .unwrap_or_else(|| fetch_hadith(today.ordinal() as usize))?;

    let cached = CachedHadith {
        date,
        hadith: hadith.clone(),
    };
    if let Ok(contents) = serde_json::to_string_pretty(&cached) {
        let _ = fs::create_dir_all(get_data_dir());
        let _ = fs::write(cache_path, contents);
    }

    println!(
        "Hadith of the Day — Sahih al-Bukhari #{}\n",
        hadith.hadithnumber
    );
    println!("{}\n", hadith.text);
    println!(
        "Reference: Book {}, Hadith {}",
        hadith.reference.book, hadith.reference.hadith
    );
    Ok(())
}

fn fetch_hadith(ordinal: usize) -> Result<Hadith, String> {
    let number = ordinal.saturating_sub(1) % 7563 + 1;
    let url = format!(
        "https://cdn.jsdelivr.net/gh/fawazahmed0/hadith-api@1/editions/eng-bukhari/{number}.json"
    );
    let response: HadithResponse = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("qari-cli/0.1.0")
        .build()
        .map_err(|error| error.to_string())?
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json)
        .map_err(|error| format!("Could not fetch today's hadith: {error}"))?;
    response
        .hadiths
        .into_iter()
        .next()
        .ok_or_else(|| "Hadith API returned no hadith".to_string())
}
