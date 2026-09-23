use crate::hadith;
use chrono::{Datelike, Local};

/// `qari hadith` alone prints the Bukhari hadith of the day; naming a book
/// lists that book, or prints one hadith when a number is given.
pub fn run(collection: Option<&str>, number: Option<u32>) -> Result<(), String> {
    match collection {
        Some(collection) => run_book(collection, number),
        None => run_today(),
    }
}

/// Reads the same bulk cache the TUI shelf uses, so one store and one
/// fetch path serve both the daily hadith and the hadith books.
fn run_today() -> Result<(), String> {
    let book = hadith::meta("bukhari").ok_or_else(|| "Unknown hadith book: bukhari".to_string())?;
    let entries = hadith::load_entries(book.id)?;
    let ordinal = Local::now().ordinal() as usize;
    let entry = entries
        .get(ordinal.saturating_sub(1) % entries.len().max(1))
        .ok_or_else(|| format!("{} has no hadiths to show", book.label))?;
    println!("Hadith of the Day — {} #{}\n", book.label, entry.number);
    println!("{}", entry.english);
    Ok(())
}

/// One hadith book: list all, or print one with Arabic and English.
fn run_book(collection: &str, number: Option<u32>) -> Result<(), String> {
    let id = collection.to_ascii_lowercase();
    let book = hadith::meta(&id)
        .ok_or_else(|| format!("Unknown hadith book: {collection} (try bukhari, muslim, abudawud, tirmidhi, nasai, ibnmajah, malik, nawawi)"))?;
    let entries = hadith::load_entries(book.id)?;
    if let Some(number) = number {
        let entry = entries
            .iter()
            .find(|entry| entry.number == number)
            .ok_or_else(|| format!("{} {number} is not available", book.label))?;
        println!("{} {}\n", book.label, entry.number);
        println!(
            "{}{}",
            entry.arabic,
            if entry.english.is_empty() {
                String::new()
            } else {
                format!("\n\n{}", entry.english)
            }
        );
        return Ok(());
    }
    println!("{} ({} hadith):\n", book.label, entries.len());
    for entry in &entries {
        let preview: String = entry.english.chars().take(72).collect();
        println!("{:>4}. {preview}", entry.number);
    }
    Ok(())
}
