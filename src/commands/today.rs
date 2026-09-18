use crate::commands::{ayah_at_index, total_ayahs};
use crate::config::Config;
use crate::quran::Surah;
use chrono::{Datelike, Local};
use std::io::{self, IsTerminal};

pub fn run(surahs: &[Surah], config: &Config) -> Result<(), String> {
    run_for_ordinal(Local::now().ordinal() as usize, surahs, config)
}

fn run_for_ordinal(ordinal: usize, surahs: &[Surah], config: &Config) -> Result<(), String> {
    let total = total_ayahs(surahs);
    if total == 0 {
        return Err("No ayahs are available".to_string());
    }
    let index = ordinal.saturating_sub(1) % total;
    let (surah, ayah) =
        ayah_at_index(surahs, index).ok_or_else(|| "Could not select today's ayah".to_string())?;
    super::read::print_ayah(surah, ayah, config, io::stdout().is_terminal());
    Ok(())
}
