use crate::commands::{ayah_at_index, total_ayahs};
use crate::config::Config;
use crate::quran::Surah;
use std::io::{self, IsTerminal};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(surahs: &[Surah], config: &Config) -> Result<(), String> {
    let total = total_ayahs(surahs);
    if total == 0 {
        return Err("No ayahs are available".to_string());
    }
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("System clock error: {error}"))?
        .subsec_nanos() as usize;
    let (surah, ayah) = ayah_at_index(surahs, seed % total)
        .ok_or_else(|| "Could not select an ayah".to_string())?;
    super::read::print_ayah(surah, ayah, config, io::stdout().is_terminal());
    Ok(())
}
