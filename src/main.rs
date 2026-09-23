mod alkotob;
mod app;
mod bookmarks;
mod collection;
mod commands;
mod config;
mod data;
mod hadith;
mod input;
mod intro;
mod prayer;
mod quran;
mod rtl;
mod search;
mod theme;
mod ui;
mod wrap;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "qari", version, about = "Read the Quran in your terminal")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Skip the startup animation
    #[arg(long, global = true)]
    no_intro: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Read a specific ayah, surah, or chapter (`tawrat gen 1:3`)
    Read {
        /// Reference: "2:255", "Al-Baqarah 2:255", "Al-Baqarah",
        /// "tawrat gen 1:3", "zabur 23", or "injil joh 3"
        reference: String,
    },
    /// Search the Quran, or a collection with --collection
    Search {
        /// Search query (minimum 2 characters)
        query: String,
        /// Collection to search: quran, tawrat, zabur, injil
        #[arg(long)]
        collection: Option<String>,
    },
    /// Print a random ayah (or chapter with a collection argument)
    Random {
        /// Collection key: quran, tawrat, zabur, injil
        collection: Option<String>,
    },
    /// Print the ayah of the day (or chapter with a collection argument)
    Today {
        /// Collection key: quran, tawrat, zabur, injil
        collection: Option<String>,
    },
    /// Show prayer times
    Pray {
        #[arg(long)]
        city: Option<String>,
        #[arg(long)]
        country: Option<String>,
    },
    /// Print the hadith of the day, or browse a hadith book
    Hadith {
        /// Hadith book to browse (bukhari, muslim, abudawud, tirmidhi,
        /// nasai, ibnmajah, malik, nawawi)
        collection: Option<String>,
        /// Hadith number to print (lists all when omitted)
        number: Option<u32>,
    },
    /// List Alkotob books (Tawrat, Zabur, Injil, …) or print one chapter
    Books {
        /// Revelation or edition id, e.g. "tawrat" or "injilen"
        revelation: Option<String>,
        /// Edition id for chapter preview, e.g. "tawrat"
        #[arg(long)]
        edition: Option<String>,
        /// Book id for chapter preview, e.g. "gen"
        #[arg(long)]
        book: Option<String>,
        /// Chapter number for chapter preview, e.g. 1
        #[arg(long)]
        chapter: Option<u32>,
    },
    /// Replay the startup animation
    Intro,
}

fn main() {
    let cli = Cli::parse();

    if let Err(error) = data::ensure_data_dir() {
        eprintln!("Cannot create qari-cli data directories: {error}");
        std::process::exit(1);
    }

    let config = config::load_config();
    let show_intro = !cli.no_intro && !config.intro_shown;

    match cli.command {
        Some(Commands::Pray { city, country }) => {
            if let Err(error) = commands::pray::run(&config, city.as_deref(), country.as_deref()) {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        Some(Commands::Hadith { collection, number }) => {
            if let Err(error) = commands::hadith::run(collection.as_deref(), number) {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        Some(Commands::Books {
            revelation,
            edition,
            book,
            chapter,
        }) => {
            if let Err(error) = commands::books::run(
                revelation.as_deref(),
                edition.as_deref(),
                book.as_deref(),
                chapter,
            ) {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        Some(Commands::Intro) => run_tui(config, true),
        Some(command) => {
            let surahs = data::load_quran().unwrap_or_else(|error| {
                eprintln!("Could not load the complete Quran: {error}");
                eprintln!("Using the bundled offline selection.");
                data::load_fallback()
            });
            let result = match command {
                Commands::Read { reference } => commands::read::run(&reference, &surahs),
                Commands::Search { query, collection } => {
                    commands::search::run(&query, &surahs, collection.as_deref())
                }
                Commands::Random { collection } => {
                    commands::random::run(&surahs, collection.as_deref())
                }
                Commands::Today { collection } => {
                    commands::today::run(&surahs, collection.as_deref())
                }
                Commands::Pray { .. }
                | Commands::Hadith { .. }
                | Commands::Books { .. }
                | Commands::Intro => unreachable!(),
            };
            if let Err(error) = result {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        None => run_tui(config, show_intro),
    }
}

fn run_tui(config: config::Config, show_intro: bool) {
    if let Err(error) = app::run(config, show_intro) {
        eprintln!("TUI error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_subcommand_parses() {
        let cli = Cli::try_parse_from(["qari", "intro"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Intro)));
    }
}
