use crate::collection::{self, CollectionId};
use crate::quran::{Ayah, Surah};
use crossterm::style::Stylize;
use std::io::{self, IsTerminal};

pub fn run(reference: &str, surahs: &[Surah]) -> Result<(), String> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err("Reference cannot be empty".to_string());
    }

    let mut parts = reference.split_whitespace();
    if let Some(first) = parts.next() {
        if let Some(collection) = CollectionId::from_key_opt(first) {
            let rest: Vec<&str> = parts.collect();
            return run_collection(collection, &rest, surahs);
        }
    }

    let selection = parse_reference(reference, surahs)?;
    let is_tty = io::stdout().is_terminal();

    match selection {
        Selection::Ayah(surah, ayah) => print_ayah(surah, ayah, is_tty),
        Selection::Surah(surah) => {
            if surah.ayahs.is_empty() {
                return Err(format!(
                    "No ayah data is available for Surah {}",
                    surah.number
                ));
            }
            for ayah in &surah.ayahs {
                print_ayah(surah, ayah, is_tty);
                if is_tty {
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// Pure parse of a collection reference (no fetch): `(book_id, chapter,
/// optional verse id)`. Single-book shelves take the number directly
/// (`zabur 23[:5]`, `bukhari 402`); multi-book shelves name the book
/// first (`tawrat gen 1`). `hadith` is the Nawawi shelf but accepts any
/// hadith book name (`hadith bukhari 402`).
fn parse_collection_ref(
    collection: CollectionId,
    rest: &[&str],
) -> Result<(&'static str, u32, Option<String>), String> {
    let (book_id, chapter_token) = match collection {
        CollectionId::Zabur => ("psa", rest.first().copied()),
        _ if collection.is_hadith() => {
            let first = rest.first().copied().unwrap_or("");
            if first.parse::<u32>().is_ok() {
                // `bukhari 402`: the rest token is the chapter number.
                (collection.hadith_id().unwrap_or(""), Some(first))
            } else {
                // `hadith bukhari 402`: name any hadith book first.
                crate::hadith::HADITH_META
                    .iter()
                    .find(|book| {
                        book.id.eq_ignore_ascii_case(first) || book.label.eq_ignore_ascii_case(first)
                    })
                    .map(|book| (book.id, rest.get(1).copied()))
                    .ok_or_else(|| {
                        format!(
                            "Unknown hadith book: {first} (try bukhari, muslim, abudawud, tirmidhi, nasai, ibnmajah, malik, nawawi)"
                        )
                    })?
            }
        }
        CollectionId::Quran => return Err("Quran references parse as surahs".to_string()),
        _ => {
            let book_token = rest.first().copied().unwrap_or("");
            let book_id = crate::collection::CollectionId::resolve_book_id(collection, book_token)
                .ok_or_else(|| {
                    format!(
                        "Unknown book: {book_token} (see `qari books {}`)",
                        collection.key()
                    )
                })?;
            (book_id, rest.get(1).copied())
        }
    };
    let chapter_token =
        chapter_token.ok_or_else(|| format!("Chapter number missing (try `{book_id} 1`)"))?;
    match chapter_token.split_once(':') {
        Some((chapter, verse)) => Ok((
            book_id,
            chapter
                .parse::<u32>()
                .map_err(|_| format!("Invalid chapter number: {chapter}"))?,
            Some(verse.to_string()),
        )),
        None => Ok((
            book_id,
            chapter_token
                .parse::<u32>()
                .map_err(|_| format!("Invalid chapter number: {chapter_token}"))?,
            None,
        )),
    }
}

/// Print one Alkotob chapter (or a single verse of it), Arabic plus
/// English where translated.
pub(crate) fn print_chapter(
    collection: CollectionId,
    book_id: &str,
    chapter: u32,
    verse: Option<&str>,
    is_tty: bool,
) -> Result<(), String> {
    let texts = collection::load_chapter_texts(collection, book_id, chapter, true)?;
    let verses: Vec<(String, String, Option<String>)> = texts
        .arabic
        .iter()
        .filter(|(id, _)| verse.is_none_or(|wanted| wanted == id))
        .map(|(id, arabic)| {
            let english = texts
                .english
                .iter()
                .find(|(english_id, _)| english_id == id)
                .map(|(_, text)| text.clone());
            (id.clone(), arabic.clone(), english)
        })
        .collect();
    if verses.is_empty() {
        return Err(match verse {
            Some(verse) => format!("Verse {verse} is not in {book_id} {chapter}"),
            None => format!("No verses found in {book_id} {chapter}"),
        });
    }

    let book_label = if collection == CollectionId::Zabur {
        format!("Psalm {chapter}")
    } else {
        format!(
            "{} {chapter}",
            collection::book_display(collection, book_id)
        )
    };
    println!(
        "{}",
        emphasize(
            &format!(
                "[{}:{book_id}:{chapter}] {book_label} · {} ({} verses)",
                collection.key(),
                collection.label(),
                verses.len()
            ),
            is_tty
        )
    );
    for (id, arabic, english) in &verses {
        println!("{id} {arabic}");
        if let Some(english) = english {
            println!("  {english}");
        }
        if is_tty {
            println!();
        }
    }
    Ok(())
}

/// `tawrat gen 1:3`, `zabur 23`, `injil joh 3`, `quran 2:255`.
/// Without a verse prints the whole chapter; with one prints the verse.
fn run_collection(collection: CollectionId, rest: &[&str], surahs: &[Surah]) -> Result<(), String> {
    if collection == CollectionId::Quran {
        return run(&rest.join(" "), surahs);
    }
    let (book_id, chapter, verse) = parse_collection_ref(collection, rest)?;
    print_chapter(
        collection,
        book_id,
        chapter,
        verse.as_deref(),
        io::stdout().is_terminal(),
    )
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

pub(crate) fn print_ayah(surah: &Surah, ayah: &Ayah, is_tty: bool) {
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

    println!("{}", emphasize(&location, is_tty));
    println!("{}  {}", emphasize("Arabic:", is_tty), ayah.arabic);
    println!("{} {}", emphasize("English:", is_tty), ayah.english);
}

fn emphasize(text: &str, is_tty: bool) -> String {
    if is_tty {
        text.bold().yellow().to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::CollectionId::{Injil, Tawrat, Zabur};

    #[test]
    fn normalizes_common_surah_spelling_punctuation() {
        assert_eq!(normalize_name("Al-Baqarah"), normalize_name("al baqarah"));
    }

    #[test]
    fn parses_collection_references() {
        assert_eq!(
            parse_collection_ref(Tawrat, &["gen", "1:3"]).unwrap(),
            ("gen", 1, Some("3".to_string()))
        );
        assert_eq!(
            parse_collection_ref(Tawrat, &["Genesis", "1"]).unwrap(),
            ("gen", 1, None)
        );
        assert_eq!(
            parse_collection_ref(Zabur, &["23"]).unwrap(),
            ("psa", 23, None)
        );
        assert_eq!(
            parse_collection_ref(Zabur, &["23:5"]).unwrap(),
            ("psa", 23, Some("5".to_string()))
        );
        assert_eq!(
            parse_collection_ref(Injil, &["joh", "3:16"]).unwrap(),
            ("joh", 3, Some("16".to_string()))
        );
        assert!(parse_collection_ref(Tawrat, &["nope", "1"]).is_err());
        assert!(parse_collection_ref(Tawrat, &["gen"]).is_err());
        assert!(parse_collection_ref(Tawrat, &["gen", "x"]).is_err());
    }
}
