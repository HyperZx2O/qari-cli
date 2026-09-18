mod app;
mod bookmarks;
mod commands;
mod config;
mod data;
mod input;
mod quran;
mod search;
mod surah_meta;
mod theme;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "islam", version, about = "Read the Quran in your terminal")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Read a specific ayah or surah
    Read {
        /// Reference: "2:255", "Al-Baqarah 2:255", or "Al-Baqarah"
        reference: String,
    },
    /// Search the Quran
    Search {
        /// Search query (minimum 2 characters)
        query: String,
    },
    /// Print a random ayah
    Random,
    /// Print the ayah of the day
    Today,
}

fn main() {
    let cli = Cli::parse();

    if let Err(error) = data::ensure_data_dir() {
        eprintln!("Cannot create islam-cli data directories: {error}");
        std::process::exit(1);
    }

    let config = config::load_config();

    match cli.command {
        Some(command) => {
            let surahs = data::load_quran(false).unwrap_or_else(|error| {
                eprintln!("Could not load the complete Quran: {error}");
                eprintln!("Using the bundled offline selection.");
                data::load_fallback()
            });
            let result = match command {
                Commands::Read { reference } => commands::read::run(&reference, &surahs, &config),
                Commands::Search { query } => commands::search::run(&query, &surahs),
                Commands::Random => commands::random::run(&surahs, &config),
                Commands::Today => commands::today::run(&surahs, &config),
            };
            if let Err(error) = result {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        None => {
            let (surahs, offline_mode) = match data::load_quran(true) {
                Ok(surahs) => (surahs, false),
                Err(error) => {
                    eprintln!("Could not load the complete Quran: {error}");
                    eprintln!("Using the bundled offline selection.");
                    (data::load_fallback(), true)
                }
            };
            if let Err(error) = app::run(surahs, config, offline_mode) {
                eprintln!("TUI error: {error}");
                std::process::exit(1);
            }
        }
    }
}
