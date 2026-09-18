use crate::config::Config;
use crate::quran::{Ayah, Surah};
use crossterm::style::Stylize;
use std::io::{self, IsTerminal};

pub fn run(reference: &str, surahs: &[Surah], config: &Config) -> Result<(), String> {
    let selection = parse_reference(reference, surahs)?;
    let is_tty = io::stdout().is_terminal();

    match selection {
        Selection::Ayah(surah, ayah) => print_ayah(surah, ayah, config, is_tty),
        Selection::Surah(surah) => {
            if surah.ayahs.is_empty() {
                return Err(format!(
                    "No ayah data is available for Surah {}",
                    surah.number
                ));
            }
            for ayah in &surah.ayahs {
                print_ayah(surah, ayah, config, is_tty);
                if is_tty {
                    println!();
                }
            }
        }
    }

    Ok(())
}

enum Selection<'a> {
    Ayah(&'a Surah, &'a Ayah),
    Surah(&'a Surah),
}

fn parse_reference<'a>(reference: &str, surahs: &'a [Surah]) -> Result<Selection<'a>, String> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err("Reference cannot be empty".to_string());
    }

    if let Some(numeric_ref) = reference
        .split_whitespace()
        .rev()
        .find(|part| part.contains(':'))
    {
        let (surah_number, ayah_number) = numeric_ref
            .split_once(':')
            .ok_or_else(|| format!("Invalid reference: {reference}"))?;
        let surah_number = surah_number
            .parse::<u8>()
            .map_err(|_| format!("Invalid surah number: {surah_number}"))?;
        let ayah_number = ayah_number
            .parse::<u16>()
            .map_err(|_| format!("Invalid ayah number: {ayah_number}"))?;
        let surah = find_surah_by_number(surahs, surah_number)?;
        let ayah = surah
            .ayahs
            .iter()
            .find(|ayah| ayah.number == ayah_number)
            .ok_or_else(|| format!("Ayah {surah_number}:{ayah_number} is not available"))?;
        return Ok(Selection::Ayah(surah, ayah));
    }

    if let Ok(number) = reference.parse::<u8>() {
        return Ok(Selection::Surah(find_surah_by_number(surahs, number)?));
    }

    let normalized = normalize_name(reference);
    surahs
        .iter()
        .find(|surah| normalize_name(&surah.name_transliterated) == normalized)
        .map(Selection::Surah)
        .ok_or_else(|| format!("Unknown surah: {reference}"))
}

fn find_surah_by_number(surahs: &[Surah], number: u8) -> Result<&Surah, String> {
    surahs
        .iter()
        .find(|surah| surah.number == number)
        .ok_or_else(|| format!("Surah {number} is not available"))
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn print_ayah(surah: &Surah, ayah: &Ayah, config: &Config, is_tty: bool) {
    let location = format!(
        "[{}:{}] {} — Ayah {} of {} ({}) · Juz {}",
        surah.number,
        ayah.number,
        surah.name_transliterated,
        ayah.number,
        surah.ayah_count,
        if surah.is_meccan { "Meccan" } else { "Medinan" },
        ayah.juz
    );

    if is_tty {
        println!("{}", location.bold().yellow());
        println!("{}  {}", "Arabic:".bold(), ayah.arabic);
        println!("{} {}", "English:".bold(), ayah.english);
        if config.language == "bn" {
            println!("{} {}", "Bengali:".bold(), ayah.bengali);
        }
    } else {
        println!("{location}");
        println!("Arabic:  {}", ayah.arabic);
        println!("English: {}", ayah.english);
        if config.language == "bn" {
            println!("Bengali: {}", ayah.bengali);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_common_surah_spelling_punctuation() {
        assert_eq!(normalize_name("Al-Baqarah"), normalize_name("al baqarah"));
    }
}
