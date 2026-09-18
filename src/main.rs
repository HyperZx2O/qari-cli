mod app;
mod bookmarks;
mod commands;
mod config;
mod data;
mod input;
mod intro;
mod prayer;
mod quran;
mod rtl;
mod search;
mod surah_meta;
mod theme;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "qari", version, about = "Read the Quran in your terminal")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Skip the startup animation
    #[arg(long, global = true)]
    no_intro: bool,

    /// Play the startup animation even if it has already been shown
    #[arg(long, global = true, conflicts_with = "no_intro")]
    intro: bool,
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
    /// Show prayer times
    Pray {
        #[arg(long)]
        city: Option<String>,
        #[arg(long)]
        country: Option<String>,
    },
    /// Print the hadith of the day
    Hadith,
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
    let show_intro = cli.intro || (!cli.no_intro && !config.intro_shown);

    match cli.command {
        Some(Commands::Pray { city, country }) => {
            if let Err(error) = commands::pray::run(&config, city.as_deref(), country.as_deref()) {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        Some(Commands::Hadith) => {
            if let Err(error) = commands::hadith::run() {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        Some(Commands::Intro) => run_tui(config, true),
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
                Commands::Pray { .. } | Commands::Hadith | Commands::Intro => unreachable!(),
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
    fn intro_flags_conflict() {
        assert!(Cli::try_parse_from(["qari", "--intro", "--no-intro"]).is_err());
    }

    #[test]
    fn intro_subcommand_parses() {
        let cli = Cli::try_parse_from(["qari", "intro"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Intro)));
    }
}
