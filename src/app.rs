use crate::config::{self, Config};
use crate::data;
use crate::input::{self, AppAction};
use crate::intro::{self, IntroState};
use crate::quran::Surah;
use crate::rtl::RtlMode;
use crate::search::{search_quran, SearchResult};
use crate::theme::Theme;
use crate::ui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;
use ratatui::DefaultTerminal;
use std::error::Error;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
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
    pub status_error: bool,
    pub quit_count: u8,
    pub(crate) quit_started: Option<Instant>,
    pub surah_list: ListState,
    pub ayah_list: ListState,
    pub search_list: ListState,
    pub bookmark_conn: Option<rusqlite::Connection>,
    pub bookmarked: bool,
    pub scripture_scroll: u16,
    pub help_scroll: u16,
    pub rtl_mode: RtlMode,
    pub(crate) status_started: Option<Instant>,
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
        let bookmarked = bookmark_conn.as_ref().is_some_and(|conn| {
            surahs
                .get(current_surah)
                .and_then(|surah| {
                    surah
                        .ayahs
                        .get(current_ayah)
                        .map(|ayah| (surah.number, ayah.number))
                })
                .is_some_and(|(surah, ayah)| crate::bookmarks::is_bookmarked(conn, surah, ayah))
        });
        let status_msg = if bookmark_conn.is_none() {
            Some("Bookmarks unavailable".to_string())
        } else {
            None
        };
        let status_started = status_msg.as_ref().map(|_| Instant::now());

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
            status_error: bookmark_conn.is_none(),
            quit_count: 0,
            quit_started: None,
            surah_list,
            ayah_list,
            search_list: ListState::default(),
            bookmark_conn,
            bookmarked,
            scripture_scroll: 0,
            help_scroll: 0,
            rtl_mode: RtlMode::detect(&config.rtl_mode),
            status_started,
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
                self.set_status(format!("Theme: {}", self.theme.label()));
            }
            AppAction::CycleLanguage => {
                self.language = self.language.next();
                self.scripture_scroll = 0;
                self.set_status(format!("Language: {}", self.language.label()));
            }
            AppAction::ToggleHelp => {
                self.show_help = !self.show_help;
                self.help_scroll = 0;
            }
            AppAction::QuitOne => {
                self.quit_count = 1;
                self.quit_started = Some(Instant::now());
                self.set_status("Press q again to quit");
            }
            AppAction::QuitConfirm => return true,
            AppAction::Noop => {}
        }
        false
    }

    pub fn expire_transients(&mut self) -> bool {
        let mut changed = false;
        if self
            .quit_started
            .is_some_and(|started| started.elapsed() > Duration::from_millis(500))
        {
            self.reset_quit();
            changed = true;
        }
        if self
            .status_started
            .is_some_and(|started| started.elapsed() > Duration::from_secs(2))
        {
            self.status_msg = None;
            self.status_error = false;
            self.status_started = None;
            changed = true;
        }
        changed
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
        if self.show_help {
            self.help_scroll = if down {
                self.help_scroll.saturating_add(1)
            } else {
                self.help_scroll.saturating_sub(1)
            };
            return;
        }

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
                self.scripture_scroll = 0;
                self.surah_list.select(Some(self.current_surah));
                self.ayah_list.select(Some(0));
                self.refresh_bookmark();
            }
            Panel::Scripture => {
                self.scripture_scroll = if down {
                    self.scripture_scroll.saturating_add(1)
                } else {
                    self.scripture_scroll.saturating_sub(1)
                };
            }
            Panel::Ayahs => {
                let len = self.current_surah().map_or(0, |surah| surah.ayahs.len());
                if len == 0 {
                    return;
                }
                self.current_ayah = if down {
                    (self.current_ayah + 1).min(len - 1)
                } else {
                    self.current_ayah.saturating_sub(1)
                };
                self.scripture_scroll = 0;
                self.ayah_list.select(Some(self.current_ayah));
                self.refresh_bookmark();
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
                self.scripture_scroll = 0;
                self.search_mode = false;
                self.search_query.clear();
                self.search_results.clear();
                self.search_list.select(None);
                self.refresh_bookmark();
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
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
            Ok(()) => self.set_status(format!("Copied {}:{}", surah.number, ayah.number)),
            Err(_) => self.set_error("Copy failed — clipboard unavailable"),
        }
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
            self.set_error("Bookmarks unavailable");
            return;
        };

        let (bookmarked, message, error) =
            if let Some(id) = crate::bookmarks::bookmark_id(conn, surah_number, ayah_number) {
                match crate::bookmarks::delete_bookmark(conn, id) {
                    Ok(()) => (
                        false,
                        format!("Removed bookmark {surah_number}:{ayah_number}"),
                        false,
                    ),
                    Err(_) => (true, "Could not remove bookmark".to_string(), true),
                }
            } else {
                match crate::bookmarks::add_bookmark(conn, surah_number, ayah_number, "", "") {
                    Ok(_) => (
                        true,
                        format!("Bookmarked {surah_number}:{ayah_number}"),
                        false,
                    ),
                    Err(_) => (false, "Could not add bookmark".to_string(), true),
                }
            };
        self.bookmarked = bookmarked;
        if error {
            self.set_error(message);
        } else {
            self.set_status(message);
        }
    }

    fn refresh_bookmark(&mut self) {
        self.bookmarked = self.bookmark_conn.as_ref().is_some_and(|conn| {
            self.surahs
                .get(self.current_surah)
                .and_then(|surah| {
                    surah
                        .ayahs
                        .get(self.current_ayah)
                        .map(|ayah| (surah.number, ayah.number))
                })
                .is_some_and(|(surah, ayah)| crate::bookmarks::is_bookmarked(conn, surah, ayah))
        });
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status_msg = Some(message.into());
        self.status_error = false;
        self.status_started = Some(Instant::now());
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.status_msg = Some(message.into());
        self.status_error = true;
        self.status_started = Some(Instant::now());
    }

    fn reset_quit(&mut self) {
        self.quit_count = 0;
        self.quit_started = None;
        if self.status_msg.as_deref() == Some("Press q again to quit") {
            self.status_msg = None;
            self.status_error = false;
            self.status_started = None;
        }
    }
}

pub fn run(config: Config, show_intro: bool) -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    let startup = wait_for_quran(&mut terminal, &config, show_intro);

    let (surahs, offline_mode) = match startup {
        Ok(Some(loaded)) => (loaded.surahs, loaded.offline_mode),
        Ok(None) => {
            ratatui::restore();
            return Ok(());
        }
        Err(error) => {
            ratatui::restore();
            return Err(error.into());
        }
    };

    if surahs.is_empty() {
        ratatui::restore();
        return Err("no Quran data is available".into());
    }

    let mut state = AppState::new(surahs, &config, offline_mode);
    let result = run_reader_loop(&mut terminal, &mut state);
    let mut updated_config = state.to_config(config);
    if show_intro {
        updated_config.intro_shown = true;
    }
    ratatui::restore();
    config::save_config(&updated_config);
    result.map_err(Into::into)
}

struct LoadedQuran {
    surahs: Vec<Surah>,
    offline_mode: bool,
}

fn spawn_quran_load() -> Receiver<LoadedQuran> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let loaded = match data::load_quran(false) {
            Ok(surahs) => LoadedQuran {
                surahs,
                offline_mode: false,
            },
            Err(_) => LoadedQuran {
                surahs: data::load_fallback(),
                offline_mode: true,
            },
        };
        let _ = sender.send(loaded);
    });
    receiver
}

fn wait_for_quran(
    terminal: &mut DefaultTerminal,
    config: &Config,
    show_intro: bool,
) -> std::io::Result<Option<LoadedQuran>> {
    let receiver = spawn_quran_load();
    let animate_intro = show_intro && !config.reduced_motion;
    let mut intro_state = IntroState::new(animate_intro);
    let mut loaded = None;
    let colors = Theme::from_config(&config.theme).colors();

    loop {
        if loaded.is_none() {
            match receiver.try_recv() {
                Ok(result) => loaded = Some(result),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    return Err(std::io::Error::other("Quran loading worker stopped"));
                }
            }
        }

        if intro_state.can_enter_reader(loaded.is_some()) {
            return Ok(loaded);
        }

        terminal.draw(|frame| intro::render(frame, &intro_state, &colors, loaded.is_some()))?;

        if event::poll(Duration::from_millis(if animate_intro { 16 } else { 50 }))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        return Ok(None);
                    }
                    intro_state.skip();
                }
            }
        }
        intro_state.tick();
    }
}

fn run_reader_loop(terminal: &mut DefaultTerminal, state: &mut AppState) -> std::io::Result<()> {
    let mut dirty = true;
    loop {
        if dirty {
            terminal.draw(|frame| ui::draw(frame, state))?;
            dirty = false;
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = input::handle_key(key, state);
                    if state.apply_action(action) {
                        return Ok(());
                    }
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        } else if state.expire_transients() {
            dirty = true;
        }
    }
}
