use crate::alkotob;
use crate::collection::CollectionId;

/// List Alkotob editions and their books (Tawrat, Zabur, Injil, …),
/// or print one chapter's verses.
/// Fetches each file once, then serves from the disk cache offline.
pub fn run(
    revelation: Option<&str>,
    edition: Option<&str>,
    book: Option<&str>,
    chapter: Option<u32>,
) -> Result<(), String> {
    if let (Some(edition), Some(book), Some(chapter)) = (edition, book, chapter) {
        let data = crate::alkotob::load_chapter(edition, book, chapter)?;
        println!(
            "{} — {} {}:{} ({} verses)",
            data.name,
            data.edition,
            data.book,
            data.number,
            data.verses.len()
        );
        for verse in &data.verses {
            println!("{} {}", verse.id, verse.content);
        }
        return Ok(());
    }

    let mut editions = crate::alkotob::load_editions()?;
    editions.sort_by(|left, right| {
        left.revelation
            .cmp(&right.revelation)
            .then(left.language.cmp(&right.language))
    });
    if let Some(wanted) = revelation {
        editions.retain(|edition| {
            edition.revelation.eq_ignore_ascii_case(wanted)
                || edition.id.eq_ignore_ascii_case(wanted)
        });
        if editions.is_empty() {
            return Err(format!("Unknown revelation or edition: {wanted}"));
        }
    }

    for edition in &editions {
        println!(
            "{} [{}] — {} ({} {})",
            edition.revelation, edition.id, edition.name, edition.language, edition.direction
        );
        match alkotob::load_books(&edition.id) {
            Ok(books) => {
                // Transliterated labels where the tables know the edition.
                let collection = CollectionId::from_edition(&edition.id);
                for book in &books {
                    let name = match collection {
                        Some(collection) => {
                            let label = collection.book_label(&book.id, &book.name);
                            if label == book.name {
                                book.name.clone()
                            } else {
                                format!("{label} · {}", book.name)
                            }
                        }
                        None => book.name.clone(),
                    };
                    println!(
                        "  {:>4}  {} ({} chapters)",
                        book.id, name, book.chapter_count
                    );
                }
            }
            Err(error) => eprintln!("  Could not list books for {}: {error}", edition.id),
        }
    }
    Ok(())
}
