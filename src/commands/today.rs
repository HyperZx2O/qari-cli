use crate::collection::CollectionId;
use crate::commands::{ayah_at_index, total_ayahs};
use crate::quran::Surah;
use chrono::{Datelike, Local};
use std::io::{self, IsTerminal};

pub fn run(surahs: &[Surah], collection: Option<&str>) -> Result<(), String> {
    let collection = CollectionId::from_cli(collection)?;
    if collection != CollectionId::Quran {
        let (book, chapter, verse) =
            super::chapter_at_ordinal(collection, Local::now().ordinal() as usize)?;
        return super::read::print_chapter(
            collection,
            &book,
            chapter,
            verse.as_deref(),
            io::stdout().is_terminal(),
        );
    }
    run_for_ordinal(Local::now().ordinal() as usize, surahs)
}

fn run_for_ordinal(ordinal: usize, surahs: &[Surah]) -> Result<(), String> {
    let total = total_ayahs(surahs);
    if total == 0 {
        return Err("No ayahs are available".to_string());
    }
    let index = ordinal.saturating_sub(1) % total;
    let (surah, ayah) =
        ayah_at_index(surahs, index).ok_or_else(|| "Could not select today's ayah".to_string())?;
    super::read::print_ayah(surah, ayah, io::stdout().is_terminal());
    Ok(())
}
