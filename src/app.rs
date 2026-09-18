use crate::config::{self, Config};
use crate::input::{self, AppAction};
use crate::quran::Surah;
use crate::search::{search_quran, SearchResult};
use crate::theme::Theme;
use crate::ui;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::widgets::ListState;
use ratatui::DefaultTerminal;
use std::error::Error;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Surahs,
    Ayahs,
    Scripture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Arabic,
    English,
    Bengali,
}

impl Language {
    pub fn next(self) -> Self {
        match self {
            Self::English => Self::Bengali,
            Self::Bengali => Self::Arabic,
            Self::Arabic => Self::English,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Arabic => "ar",
            Self::English => "en",
            Self::Bengali => "bn",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Arabic => "AR",
            Self::English => "EN",
            Self::Bengali => "BN",
        }
    }

    fn from_config(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "ar" => Self::Arabic,
            "bn" => Self::Bengali,
            _ => Self::English,
        }
    }
}

pub struct AppState {
    pub surahs: Vec<Surah>,
    pub current_surah: usize,
    pub current_ayah: usize,
    pub active_panel: Panel,
    pub language: Language,
    pub theme: Theme,
    pub search_mode: bool,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub show_help: bool,
    pub offline_mode: bool,
    pub status_msg: Option<String>,
    pub quit_count: u8,
    pub(crate) quit_started: Option<Instant>,
    pub surah_list: ListState,
    pub ayah_list: ListState,
    pub search_list: ListState,
    pub bookmark_conn: Option<rusqlite::Connection>,
}

impl AppState {
    pub fn new(surahs: Vec<Surah>, config: &Config, offline_mode: bool) -> Self {
        let current_surah = surahs
            .iter()
            .position(|surah| surah.number == config.last_surah)
            .unwrap_or(0);
        let current_ayah = surahs
            .get(current_surah)
            .and_then(|surah| {
                surah
                    .ayahs
                    .iter()
                    .position(|ayah| ayah.number == config.last_ayah)
            })
            .unwrap_or(0);

        let mut surah_list = ListState::default();
        surah_list.select(Some(current_surah));
        let mut ayah_list = ListState::default();
        ayah_list.select(Some(current_ayah));

        let bookmark_conn = crate::bookmarks::init_db().ok();
        let status_msg = if bookmark_conn.is_none() {
            Some("Bookmarks unavailable".to_string())
        } else {
            None
        };

        Self {
            surahs,
            current_surah,
            current_ayah,
            active_panel: Panel::Surahs,
            language: Language::from_config(&config.language),
            theme: Theme::from_config(&config.theme),
            search_mode: false,
            search_query: String::new(),
            search_results: Vec::new(),
            show_help: false,
            offline_mode,
            status_msg,
            quit_count: 0,
            quit_started: None,
            surah_list,
            ayah_list,
            search_list: ListState::default(),
            bookmark_conn,
        }
    }

    pub fn current_surah(&self) -> Option<&Surah> {
        self.surahs.get(self.current_surah)
    }

    pub fn apply_action(&mut self, action: AppAction) -> bool {
        if !matches!(action, AppAction::QuitOne | AppAction::QuitConfirm) {
            self.reset_quit();
        }

        match action {
            AppAction::PanelLeft => {
                self.active_panel = match self.active_panel {
                    Panel::Surahs => Panel::Surahs,
                    Panel::Ayahs => Panel::Surahs,
                    Panel::Scripture => Panel::Ayahs,
                };
            }
            AppAction::PanelRight => {
                self.active_panel = match self.active_panel {
                    Panel::Surahs => Panel::Ayahs,
                    Panel::Ayahs | Panel::Scripture => Panel::Scripture,
                };
            }
            AppAction::SelectUp => self.move_selection(false),
            AppAction::SelectDown => self.move_selection(true),
            AppAction::Select => self.select(),
            AppAction::OpenSearch => {
                self.search_mode = true;
                self.search_query.clear();
                self.search_results.clear();
                self.search_list.select(None);
            }
            AppAction::CloseSearch => {
                self.search_mode = false;
                self.search_query.clear();
                self.search_results.clear();
                self.search_list.select(None);
                self.reset_quit();
            }
            AppAction::SearchInput(character) => {
                self.search_query.push(character);
                self.update_search();
            }
            AppAction::SearchBackspace => {
                self.search_query.pop();
                self.update_search();
            }
            AppAction::CopyAyah => self.copy_ayah(),
            AppAction::ToggleBookmark => self.toggle_bookmark(),
            AppAction::CycleTheme => {
                self.theme = self.theme.next();
                self.status_msg = Some(format!("Theme: {}", self.theme.label()));
            }
            AppAction::CycleLanguage => {
                self.language = self.language.next();
                self.status_msg = Some(format!("Language: {}", self.language.label()));
            }
            AppAction::ToggleHelp => self.show_help = !self.show_help,
            AppAction::QuitOne => {
                self.quit_count = 1;
                self.quit_started = Some(Instant::now());
                self.status_msg = Some("Press q again to quit".to_string());
            }
            AppAction::QuitConfirm => return true,
            AppAction::Noop => {}
        }
        false
    }

    pub fn expire_quit(&mut self) {
        if self
            .quit_started
            .is_some_and(|started| started.elapsed() > Duration::from_millis(500))
        {
            self.reset_quit();
        }
    }

    pub fn to_config(&self, mut config: Config) -> Config {
        if let (Some(surah), Some(ayah)) = (
            self.surahs.get(self.current_surah),
            self.surahs
                .get(self.current_surah)
                .and_then(|surah| surah.ayahs.get(self.current_ayah)),
        ) {
            config.last_surah = surah.number;
            config.last_ayah = ayah.number;
        }
        config.language = self.language.code().to_string();
        config.theme = self.theme.label().to_ascii_lowercase();
        config
    }

    fn move_selection(&mut self, down: bool) {
        if self.search_mode {
            let len = self.search_results.len();
            if len == 0 {
                return;
            }
            let current = self.search_list.selected().unwrap_or(0);
            let next = if down {
                (current + 1).min(len - 1)
            } else {
                current.saturating_sub(1)
            };
            self.search_list.select(Some(next));
            return;
        }

        match self.active_panel {
            Panel::Surahs => {
                if self.surahs.is_empty() {
                    return;
                }
                self.current_surah = if down {
                    (self.current_surah + 1).min(self.surahs.len() - 1)
                } else {
                    self.current_surah.saturating_sub(1)
                };
                self.current_ayah = 0;
                self.surah_list.select(Some(self.current_surah));
                self.ayah_list.select(Some(0));
            }
            Panel::Ayahs | Panel::Scripture => {
                let len = self.current_surah().map_or(0, |surah| surah.ayahs.len());
                if len == 0 {
                    return;
                }
                self.current_ayah = if down {
                    (self.current_ayah + 1).min(len - 1)
                } else {
                    self.current_ayah.saturating_sub(1)
                };
                self.ayah_list.select(Some(self.current_ayah));
            }
        }
    }

    fn select(&mut self) {
        if self.search_mode {
            let Some(result) = self
                .search_list
                .selected()
                .and_then(|index| self.search_results.get(index))
                .cloned()
            else {
                return;
            };
            if let Some(surah_index) = self
                .surahs
                .iter()
                .position(|surah| surah.number == result.surah_number)
            {
                self.current_surah = surah_index;
                self.current_ayah = self.surahs[surah_index]
                    .ayahs
                    .iter()
                    .position(|ayah| ayah.number == result.ayah_number)
                    .unwrap_or(0);
                self.surah_list.select(Some(self.current_surah));
                self.ayah_list.select(Some(self.current_ayah));
                self.active_panel = Panel::Scripture;
                self.search_mode = false;
                self.search_query.clear();
                self.search_results.clear();
                self.search_list.select(None);
            }
            return;
        }

        self.active_panel = match self.active_panel {
            Panel::Surahs => Panel::Ayahs,
            Panel::Ayahs => Panel::Scripture,
            Panel::Scripture => Panel::Scripture,
        };
    }

    fn update_search(&mut self) {
        self.search_results = search_quran(&self.search_query, &self.surahs);
        self.search_list
            .select((!self.search_results.is_empty()).then_some(0));
    }

    fn copy_ayah(&mut self) {
        let Some(surah) = self.surahs.get(self.current_surah) else {
            return;
        };
        let Some(ayah) = surah.ayahs.get(self.current_ayah) else {
            return;
        };
        let translation = match self.language {
            Language::Arabic => String::new(),
            Language::English => format!("\n{}", ayah.english),
            Language::Bengali => format!("\n{}", ayah.bengali),
        };
        let text = format!(
            "{} {}:{}\n{}{}",
            surah.name_transliterated, surah.number, ayah.number, ayah.arabic, translation
        );
        self.status_msg =
            match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
                Ok(()) => Some(format!("Copied {}:{}", surah.number, ayah.number)),
                Err(_) => Some("Copy failed — clipboard unavailable".to_string()),
            };
    }

    fn toggle_bookmark(&mut self) {
        let Some(surah) = self.surahs.get(self.current_surah) else {
            return;
        };
        let Some(ayah) = surah.ayahs.get(self.current_ayah) else {
            return;
        };
        let (surah_number, ayah_number) = (surah.number, ayah.number);
        let Some(conn) = &self.bookmark_conn else {
            self.status_msg = Some("Bookmarks unavailable".to_string());
            return;
        };

        if let Some(id) = crate::bookmarks::bookmark_id(conn, surah_number, ayah_number) {
            self.status_msg = match crate::bookmarks::delete_bookmark(conn, id) {
                Ok(()) => Some(format!("Removed bookmark {surah_number}:{ayah_number}")),
                Err(_) => Some("Could not remove bookmark".to_string()),
            };
        } else {
            self.status_msg =
                match crate::bookmarks::add_bookmark(conn, surah_number, ayah_number, "", "") {
                    Ok(_) => Some(format!("Bookmarked {surah_number}:{ayah_number}")),
                    Err(_) => Some("Could not add bookmark".to_string()),
                };
        }
    }

    fn reset_quit(&mut self) {
        self.quit_count = 0;
        self.quit_started = None;
        if self.status_msg.as_deref() == Some("Press q again to quit") {
            self.status_msg = None;
        }
    }
}

pub fn run(surahs: Vec<Surah>, config: Config, offline_mode: bool) -> Result<(), Box<dyn Error>> {
    if surahs.is_empty() {
        return Err("no Quran data is available".into());
    }

    let mut terminal = ratatui::init();
    let mut state = AppState::new(surahs, &config, offline_mode);
    let result = run_loop(&mut terminal, &mut state);
    let updated_config = state.to_config(config);
    ratatui::restore();
    config::save_config(&updated_config);
    result.map_err(Into::into)
}

fn run_loop(terminal: &mut DefaultTerminal, state: &mut AppState) -> std::io::Result<()> {
    loop {
        state.expire_quit();
        terminal.draw(|frame| ui::draw(frame, state))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let action = input::handle_key(key, state);
                    if state.apply_action(action) {
                        return Ok(());
                    }
                }
            }
        }
    }
}
