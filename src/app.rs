use crate::alkotob::BookMeta;
use crate::collection::{ChapterTexts, CollectionId, FlatUnit};
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
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

/// Visible columns. Panel names are historic: `Surahs` is the Books
/// column (the single library shelf), `Ayahs` is the Units column
/// (surahs / flattened chapters / psalms / hadiths), `Scripture` is the
/// full-unit reading pane with a verse cursor.
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
    Hebrew,
    Greek,
}

impl Language {
    pub fn next(self) -> Self {
        match self {
            Self::English => Self::Arabic,
            Self::Arabic => Self::English,
            Self::Hebrew => Self::Arabic,
            Self::Greek => Self::English,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Arabic => "ar",
            Self::English => "en",
            Self::Hebrew => "he",
            Self::Greek => "el",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Arabic => "AR",
            Self::English => "EN",
            Self::Hebrew => "HE",
            Self::Greek => "EL",
        }
    }

    fn from_config(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "ar" => Self::Arabic,
            "he" => Self::Hebrew,
            "el" => Self::Greek,
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
    pub collection: CollectionId,
    pub books: Vec<BookMeta>,
    pub current_book: usize,
    pub current_chapter: usize,
    /// Column 2 rows for non-Quran shelves, flattened with book prefixes.
    pub units: Vec<FlatUnit>,
    pub current_unit: usize,
    pub library_list: ListState,
    pub unit_list: ListState,
    pub chapter_cache: HashMap<(String, u32), ChapterTexts>,
    pub chapter_error: Option<String>,
    /// Parsed hadith bulks by book id, shared by reference: re-opening a
    /// book reuses its entries instead of re-parsing its two JSON files.
    /// Never pruned — eight books worst case, a few dozen MB in total.
    pub(crate) hadith_bulk: HashMap<String, crate::hadith::HadithBulk>,
    /// Set when the verse cursor moves; the next render scrolls the
    /// reading pane to that verse instead of guessing line math blind.
    pub scroll_to_verse: bool,
    pub juz_mode: bool,
    pub current_juz: u8,
    pub show_transliteration: bool,
    pub translit_cache: HashMap<u8, Vec<(String, String)>>,
    pub bookmark_error: Option<String>,
    pub bookmarked: bool,
    pub scripture_scroll: u16,
    pub help_scroll: u16,
    /// Verse focus modal (`f`): the verse under the cursor with
    /// transliteration and highlighted translation.
    pub show_verse: bool,
    pub verse_scroll: u16,
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

        let initial_bookmarks = crate::bookmarks::load();
        let bookmark_error = initial_bookmarks.as_ref().err().cloned();
        let bookmarked = initial_bookmarks.is_ok_and(|bookmarks| {
            surahs
                .get(current_surah)
                .and_then(|surah| {
                    surah
                        .ayahs
                        .get(current_ayah)
                        .map(|ayah| crate::bookmarks::quran_key(surah.number, ayah.number))
                })
                .is_some_and(|key| bookmarks.contains(&key))
        });
        let status_msg = bookmark_error.as_deref().map(|reason| {
            crate::output::sanitize_terminal_text(&format!("Bookmarks unavailable ({reason})"))
        });
        let status_started = status_msg.as_ref().map(|_| Instant::now());

        let collection = CollectionId::from_key(&config.collection);
        let library_index = CollectionId::library()
            .iter()
            .position(|id| *id == collection)
            .unwrap_or(0);
        let mut library_list = ListState::default();
        library_list.select(Some(library_index));
        let mut state = Self {
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
            status_error: bookmark_error.is_some(),
            quit_count: 0,
            quit_started: None,
            surah_list,
            ayah_list,
            search_list: ListState::default(),
            collection,
            books: Vec::new(),
            current_book: 0,
            current_chapter: config.last_chapter.max(1) as usize,
            units: Vec::new(),
            current_unit: 0,
            library_list,
            unit_list: ListState::default(),
            chapter_cache: HashMap::new(),
            chapter_error: None,
            hadith_bulk: HashMap::new(),
            scroll_to_verse: true,
            juz_mode: false,
            current_juz: 1,
            show_transliteration: config.transliteration,
            translit_cache: HashMap::new(),
            bookmark_error,
            bookmarked,
            scripture_scroll: 0,
            help_scroll: 0,
            show_verse: false,
            verse_scroll: 0,
            rtl_mode: RtlMode::detect(&config.rtl_mode),
            status_started,
        };
        if collection != CollectionId::Quran {
            state.enter_collection(
                collection,
                Some(config.last_book.clone()),
                state.current_chapter,
            );
        }
        state
    }

    /// Switch the library shelf, loading its book index and flattening it
    /// into column-2 units. Failures fall back to the Quran with an
    /// explanatory status instead of a broken shelf.
    pub fn enter_collection(
        &mut self,
        id: CollectionId,
        saved_book: Option<String>,
        saved_chapter: usize,
    ) {
        self.collection = id;
        self.books.clear();
        self.units.clear();
        self.current_unit = 0;
        self.current_ayah = 0;
        self.chapter_cache.clear();
        self.chapter_error = None;
        self.current_book = 0;
        self.current_chapter = saved_chapter.max(1);
        self.unit_list.select(None);
        self.scripture_scroll = 0;
        self.scroll_to_verse = true;
        self.search_mode = false;
        self.search_query.clear();
        self.search_results.clear();
        self.search_list.select(None);
        self.library_list.select(
            CollectionId::library()
                .iter()
                .position(|library| *library == id),
        );
        if id == CollectionId::Quran {
            self.surah_list.select(Some(self.current_surah));
            self.ayah_list.select(Some(self.current_ayah));
            self.ensure_transliteration();
            self.refresh_bookmark();
            return;
        }
        // Arabic-only collections reset the language; Tanakh and Greek
        // select their own script; Quran, Injil and every hadith book
        // keep it.
        self.language = match id {
            CollectionId::Tawrat | CollectionId::Zabur => Language::Arabic,
            CollectionId::Tanakh => Language::Hebrew,
            CollectionId::Greek => Language::Greek,
            _ => self.language,
        };
        if id.is_hadith() {
            return self.enter_hadith();
        }
        let edition = id.arabic_edition().unwrap_or(id.key());
        match crate::alkotob::load_books(edition) {
            Err(error) => {
                self.collection = CollectionId::Quran;
                self.library_list.select(Some(0));
                self.set_error(format!(
                    "Could not load {} ({error}); staying in Quran",
                    id.label()
                ));
                self.refresh_bookmark();
            }
            Ok(books) => {
                if books.is_empty() {
                    self.collection = CollectionId::Quran;
                    self.library_list.select(Some(0));
                    self.set_error(format!("No books in {}; staying in Quran", id.label()));
                    self.refresh_bookmark();
                    return;
                }
                self.units = crate::collection::flat_units(id, &books);
                self.books = books;
                self.land_on_unit(saved_book.as_deref(), self.current_chapter);
                self.ensure_chapter();
                self.refresh_bookmark();
            }
        }
    }

    /// Point column 2 at the saved `(book, chapter)` unit, or the first
    /// unit when the saved one no longer exists.
    fn land_on_unit(&mut self, saved_book: Option<&str>, saved_chapter: usize) {
        let position = saved_book
            .and_then(|wanted| {
                self.units
                    .iter()
                    .position(|unit| unit.book_id == wanted && unit.chapter == saved_chapter as u32)
            })
            .or_else(|| {
                self.units
                    .iter()
                    .position(|unit| unit.chapter == saved_chapter as u32)
            })
            .unwrap_or(0);
        self.select_unit(position);
    }

    /// Select a flattened unit and derive the book/chapter behind it.
    fn select_unit(&mut self, position: usize) {
        let position = position.min(self.units.len().saturating_sub(1));
        self.current_unit = position;
        if let Some(unit) = self.units.get(position) {
            self.current_chapter = unit.chapter as usize;
            if let Some(book) = self.books.iter().position(|book| book.id == unit.book_id) {
                self.current_book = book;
            }
        }
        self.current_ayah = 0;
        self.scripture_scroll = 0;
        self.scroll_to_verse = true;
        self.unit_list.select(Some(position));
        self.ayah_list.select(Some(0));
    }

    /// Enter one hadith book: its single static book whose entries load
    /// as a whole (two bulk files), then fan out into per-number chapters
    /// so every downstream path (panels, search, copy, bookmarks) works
    /// unchanged. Units carry the entry's own hadith number, so dropped
    /// or renumbered rows simply never appear. Navigation below never
    /// fetches.
    fn enter_hadith(&mut self) {
        let Some(book_id) = self.collection.hadith_id() else {
            return;
        };
        match crate::hadith::entry_rows(book_id, &mut self.hadith_bulk) {
            Err(error) => {
                self.collection = CollectionId::Quran;
                self.library_list.select(Some(0));
                self.set_error(format!(
                    "Could not load {book_id} ({error}); staying in Quran"
                ));
                self.refresh_bookmark();
            }
            Ok(entries) => {
                let total = entries.len().max(1);
                let Some(book) = crate::hadith::meta(book_id) else {
                    return;
                };
                self.books = vec![crate::alkotob::BookMeta {
                    id: book_id.to_string(),
                    name: book.label.to_string(),
                    chapter_count: total as u32,
                }];
                self.units = crate::collection::flat_units(self.collection, &self.books);
                for entry in entries.iter() {
                    let id = entry.number.to_string();
                    self.chapter_cache.insert(
                        (book_id.to_string(), entry.number),
                        ChapterTexts {
                            arabic: vec![(id.clone(), entry.arabic.clone())],
                            english: if entry.english.is_empty() {
                                Vec::new()
                            } else {
                                vec![(id, entry.english.clone())]
                            },
                        },
                    );
                }
                self.current_book = 0;
                self.current_chapter = self.current_chapter.clamp(1, total);
                self.current_ayah = 0;
                self.current_unit = (self.current_chapter - 1).min(total - 1);
                self.unit_list.select(Some(self.current_unit));
                self.ayah_list.select(Some(0));
                self.scroll_to_verse = true;
                self.refresh_bookmark();
            }
        }
    }

    /// Fetch the current surah's transliteration unless cached. Failures
    /// revert the toggle with a reason instead of rendering gaps.
    fn ensure_transliteration(&mut self) {
        if !self.show_transliteration || self.collection != CollectionId::Quran {
            return;
        }
        let Some(number) = self
            .surahs
            .get(self.current_surah)
            .map(|surah| surah.number)
        else {
            return;
        };
        if self.translit_cache.contains_key(&number) {
            return;
        }
        match crate::alkotob::load_chapter("qeng63", "quran", number as u32) {
            Ok(chapter) => {
                self.translit_cache.insert(
                    number,
                    chapter
                        .verses
                        .into_iter()
                        .map(|verse| (verse.id, verse.content))
                        .collect(),
                );
            }
            Err(error) => {
                self.show_transliteration = false;
                self.set_error(format!("Transliteration unavailable ({error})"));
            }
        }
    }

    /// Re-read one cached chapter with English and store it: the
    /// stale-cache upgrade shared by `ensure_chapter` and the verse
    /// modal. A failed upgrade keeps the Arabic-only rows.
    fn upgrade_chapter_english(&mut self, key: &(String, u32)) {
        if let Ok(texts) =
            crate::collection::load_chapter_texts(self.collection, &key.0, key.1, true)
        {
            self.chapter_cache.insert(key.clone(), texts);
            self.chapter_error = None;
        }
    }

    /// True when a cached chapter needs a re-read for English: cached
    /// Arabic-only while English is asked for and the shelf has some.
    /// Pure so the upgrade truth table stays unit-testable — the fetch
    /// itself needs disk and network.
    pub(crate) fn needs_english_upgrade(
        collection: CollectionId,
        cached_english_empty: bool,
        want_english: bool,
    ) -> bool {
        cached_english_empty
            && want_english
            && (collection.english_edition().is_some() || collection.is_hadith())
    }

    /// Fetch the current chapter's texts into the cache unless already
    /// there. No-op for the Quran, whose corpus is fully loaded. A
    /// chapter first cached in Arabic is re-read once English is asked
    /// for (the cache key carries no language); a failed upgrade keeps
    /// serving the Arabic-only rows instead of erroring.
    fn ensure_chapter(&mut self) {
        if self.collection == CollectionId::Quran {
            return;
        }
        let Some(book_id) = self
            .books
            .get(self.current_book)
            .map(|book| book.id.clone())
        else {
            self.chapter_error = Some("No book selected".to_string());
            return;
        };
        let key = (book_id.clone(), self.current_chapter as u32);
        if self.chapter_cache.get(&key).is_some_and(|texts| {
            Self::needs_english_upgrade(
                self.collection,
                texts.english.is_empty(),
                self.language == Language::English,
            )
        }) {
            self.upgrade_chapter_english(&key);
            return;
        }
        if self.chapter_cache.contains_key(&key) {
            self.chapter_error = None;
            return;
        }
        match crate::collection::load_chapter_texts(
            self.collection,
            &book_id,
            self.current_chapter as u32,
            self.language == Language::English,
        ) {
            Ok(texts) => {
                self.chapter_cache.insert(key, texts);
                self.chapter_error = None;
            }
            Err(error) => self.chapter_error = Some(error),
        }
    }

    /// Prepare the verse focus modal (`f`) for the verse under the
    /// cursor. True opens it; false leaves it closed. Quran verses are
    /// always in memory; other shelves need their chapter cached (opening
    /// never touches the network). A chapter first cached in Arabic is
    /// re-read with English so the modal shows the translation even
    /// before `v` is pressed; a failed upgrade keeps the Arabic-only
    /// rows and their honest `(AR)` marker.
    fn open_verse_modal(&mut self) -> bool {
        if self.collection == CollectionId::Quran {
            let Some(surah) = self.surahs.get(self.current_surah) else {
                return false;
            };
            let number = surah.number;
            if surah.ayahs.get(self.current_ayah).is_none() {
                return false;
            }
            // Warm the transliteration cache without flipping the `i`
            // toggle behind the user's back.
            if !self.translit_cache.contains_key(&number) {
                let was_on = self.show_transliteration;
                self.show_transliteration = true;
                self.ensure_transliteration();
                self.show_transliteration = was_on && self.translit_cache.contains_key(&number);
            }
            return true;
        }
        let Some(book_id) = self
            .books
            .get(self.current_book)
            .map(|book| book.id.clone())
        else {
            return false;
        };
        let key = (book_id, self.current_chapter as u32);
        let Some(cached) = self.chapter_cache.get(&key) else {
            return false;
        };
        if Self::needs_english_upgrade(self.collection, cached.english.is_empty(), true) {
            self.upgrade_chapter_english(&key);
        }
        true
    }

    /// `(verse_id, text)` of the verse under the cursor: the Quran's ayah
    /// number plus its Uthmani text, or the open unit's cached row on any
    /// other shelf (Tawrat, Zabur, Injil, Tanakh, Greek, Hadith).
    pub(crate) fn active_verse(&self) -> Option<(String, String)> {
        if self.collection == CollectionId::Quran {
            let ayah = self
                .surahs
                .get(self.current_surah)?
                .ayahs
                .get(self.current_ayah)?;
            return Some((ayah.number.to_string(), ayah.arabic.clone()));
        }
        self.chapter_texts()?
            .arabic
            .into_iter()
            .nth(self.current_ayah)
    }

    /// English rendering of one verse of the open unit, when translated.
    pub(crate) fn active_verse_english(&self, verse_id: &str) -> Option<String> {
        self.chapter_texts()?
            .english
            .iter()
            .find(|(id, _)| id == verse_id)
            .map(|(_, text)| text.clone())
    }

    /// Singular unit noun for rows and headings: Hadith vs Psalm.
    pub(crate) fn unit_label(&self) -> &'static str {
        if self.collection.is_hadith() {
            "Hadith"
        } else {
            "Psalm"
        }
    }

    /// Cached texts for the current non-Quran chapter, if loaded.
    pub(crate) fn chapter_texts(&self) -> Option<crate::collection::ChapterTexts> {
        let book_id = self
            .books
            .get(self.current_book)
            .map(|book| book.id.clone())?;
        self.chapter_cache
            .get(&(book_id, self.current_chapter as u32))
            .cloned()
    }

    /// Transliterated label for the current book, e.g. "Genesis".
    pub(crate) fn current_book_label(&self) -> String {
        self.books
            .get(self.current_book)
            .map(|book| self.collection.book_label(&book.id, &book.name))
            .unwrap_or_default()
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
            AppAction::SelectUp => {
                if self.show_verse {
                    self.verse_scroll = self.verse_scroll.saturating_sub(1);
                } else {
                    self.move_selection(false);
                }
            }
            AppAction::SelectDown => {
                if self.show_verse {
                    self.verse_scroll = self.verse_scroll.saturating_add(1);
                } else {
                    self.move_selection(true);
                }
            }
            AppAction::Select => self.select(),
            AppAction::OpenSearch => {
                self.search_mode = true;
                self.search_query.clear();
                self.search_results.clear();
                self.search_list.select(None);
            }
            AppAction::CloseSearch => self.close_search(),
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
                // Each collection sticks to the languages it actually has.
                self.language = match self.collection {
                    CollectionId::Tawrat | CollectionId::Zabur => Language::Arabic,
                    CollectionId::Tanakh => Language::Hebrew,
                    CollectionId::Greek => Language::Greek,
                    _ => self.language.next(),
                };
                self.scripture_scroll = 0;
                self.ensure_chapter();
                self.set_status(format!("Language: {}", self.language.label()));
            }
            AppAction::CycleTransliteration => {
                if self.collection != CollectionId::Quran {
                    self.set_status("Transliteration is Quran-only for now");
                } else {
                    self.show_transliteration = !self.show_transliteration;
                    self.scripture_scroll = 0;
                    self.ensure_transliteration();
                    self.set_status(format!(
                        "Transliteration: {}",
                        if self.show_transliteration {
                            "on"
                        } else {
                            "off"
                        }
                    ));
                }
            }
            AppAction::ToggleJuzMode => {
                if self.collection != CollectionId::Quran {
                    self.set_status("Juz navigation is Quran-only for now");
                } else {
                    self.juz_mode = !self.juz_mode;
                    if self.juz_mode {
                        if let Some(juz) = self
                            .surahs
                            .get(self.current_surah)
                            .and_then(|surah| surah.ayahs.get(self.current_ayah))
                            .map(|ayah| ayah.juz)
                        {
                            self.current_juz = juz.max(1);
                        }
                        self.surah_list
                            .select(Some((self.current_juz - 1) as usize));
                        self.ayah_list.select(Some(0));
                    } else {
                        self.surah_list.select(Some(self.current_surah));
                        self.ayah_list.select(Some(self.current_ayah));
                    }
                    self.scripture_scroll = 0;
                }
            }
            AppAction::ToggleHelp => {
                self.show_help = !self.show_help;
                self.help_scroll = 0;
            }
            AppAction::ToggleVerse => {
                if self.show_verse {
                    self.show_verse = false;
                } else if self.open_verse_modal() {
                    self.show_verse = true;
                    self.verse_scroll = 0;
                }
            }
            AppAction::QuitOne => {
                self.quit_count = 1;
                self.quit_started = Some(Instant::now());
                self.set_status("Press q again to quit");
            }
            AppAction::QuitConfirm => return true,
            // Redraw is handled by the event loop (terminal.clear());
            // state itself is unchanged.
            AppAction::Redraw | AppAction::Noop => {}
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
        if self.collection == CollectionId::Quran {
            if let (Some(surah), Some(ayah)) = (
                self.surahs.get(self.current_surah),
                self.surahs
                    .get(self.current_surah)
                    .and_then(|surah| surah.ayahs.get(self.current_ayah)),
            ) {
                config.last_surah = surah.number;
                config.last_ayah = ayah.number;
            }
        }
        config.collection = self.collection.key().to_string();
        config.transliteration = self.show_transliteration;
        config.last_book = self
            .books
            .get(self.current_book)
            .map(|book| book.id.clone())
            .unwrap_or_default();
        config.last_chapter = self.current_chapter as u32;
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
            // Column 1: the library shelf itself. Moving here switches
            // the book under columns 2 and 3; there is no mode key.
            Panel::Surahs => {
                if self.juz_mode && self.collection == CollectionId::Quran {
                    let current = (self.current_juz - 1).min(29);
                    let next = if down {
                        (current + 1).min(29)
                    } else {
                        current.saturating_sub(1)
                    };
                    self.current_juz = next + 1;
                    self.scripture_scroll = 0;
                    self.surah_list.select(Some(next as usize));
                    self.ayah_list.select(Some(0));
                    return;
                }
                let library = CollectionId::library();
                let current = self
                    .library_list
                    .selected()
                    .unwrap_or(0)
                    .min(library.len() - 1);
                let next = if down {
                    (current + 1).min(library.len() - 1)
                } else {
                    current.saturating_sub(1)
                };
                self.library_list.select(Some(next));
                if library[next] != self.collection {
                    self.enter_collection(library[next], None, 1);
                }
            }
            // Column 3: the verse cursor inside the open unit. The pane
            // follows on the next render; every step stays bookmarkable.
            Panel::Scripture => self.move_verse(down),
            // Column 2: the unit list (surahs, flattened chapters,
            // psalms, hadiths).
            Panel::Ayahs => {
                if self.juz_mode && self.collection == CollectionId::Quran {
                    let entries = crate::quran::juz_entries(&self.surahs, self.current_juz);
                    if entries.is_empty() {
                        return;
                    }
                    let current = self
                        .ayah_list
                        .selected()
                        .unwrap_or(0)
                        .min(entries.len() - 1);
                    let next = if down {
                        (current + 1).min(entries.len() - 1)
                    } else {
                        current.saturating_sub(1)
                    };
                    self.ayah_list.select(Some(next));
                    self.scripture_scroll = 0;
                    return;
                }
                if self.collection == CollectionId::Quran {
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
                    self.scroll_to_verse = true;
                    self.surah_list.select(Some(self.current_surah));
                    self.ayah_list.select(Some(0));
                    self.ensure_transliteration();
                    self.refresh_bookmark();
                    return;
                }
                if self.units.is_empty() {
                    return;
                }
                let next = if down {
                    (self.current_unit + 1).min(self.units.len() - 1)
                } else {
                    self.current_unit.saturating_sub(1)
                };
                self.select_unit(next);
                self.ensure_chapter();
                self.refresh_bookmark();
            }
        }
    }

    /// Step the verse cursor inside the open unit (column 3). Quran,
    /// Zabur and single-verse Hadith address verses directly; chapter
    /// shelves index into the cached chapter's Arabic rows.
    fn move_verse(&mut self, down: bool) {
        let total = self.verse_count();
        if total == 0 {
            return;
        }
        let current = self.current_ayah.min(total - 1);
        self.current_ayah = if down {
            (current + 1).min(total - 1)
        } else {
            current.saturating_sub(1)
        };
        self.scroll_to_verse = true;
        self.ayah_list.select(Some(self.current_ayah));
        self.refresh_bookmark();
    }

    /// Verses in the open unit under the cursor.
    fn verse_count(&self) -> usize {
        if self.collection == CollectionId::Quran {
            return self
                .surahs
                .get(self.current_surah)
                .map(|surah| surah.ayahs.len())
                .unwrap_or(0);
        }
        self.chapter_texts()
            .map(|texts| texts.arabic.len())
            .unwrap_or(0)
    }

    /// Verse ids in cursor order, for landing search hits on the verse.
    fn verse_ids(&self) -> Vec<String> {
        if self.collection == CollectionId::Quran {
            return self
                .surahs
                .get(self.current_surah)
                .map(|surah| {
                    surah
                        .ayahs
                        .iter()
                        .map(|ayah| ayah.number.to_string())
                        .collect()
                })
                .unwrap_or_default();
        }
        self.chapter_texts()
            .map(|texts| texts.arabic.into_iter().map(|(id, _)| id).collect())
            .unwrap_or_default()
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
            if result.collection == "quran" {
                // Quran hits jump home before landing.
                if self.collection != CollectionId::Quran {
                    self.enter_collection(CollectionId::Quran, None, 1);
                }
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
                    self.close_search();
                    self.ensure_transliteration();
                    self.refresh_bookmark();
                }
                return;
            }
            // Collection hits land on their shelf: switch books, open the
            // unit, and drop the verse cursor on the hit verse.
            let target =
                CollectionId::from_key_opt(&result.collection).unwrap_or(CollectionId::Quran);
            if target != self.collection {
                self.enter_collection(
                    target,
                    Some(result.book_id.clone()),
                    result.chapter as usize,
                );
            } else if let Some(unit) = self
                .units
                .iter()
                .position(|unit| unit.book_id == result.book_id && unit.chapter == result.chapter)
            {
                self.select_unit(unit);
                self.ensure_chapter();
            }
            let verse = self
                .verse_ids()
                .iter()
                .position(|id| *id == result.verse_id)
                .unwrap_or(0);
            self.current_ayah = verse;
            self.scroll_to_verse = true;
            self.ayah_list.select(Some(verse));
            self.active_panel = Panel::Scripture;
            self.scripture_scroll = 0;
            self.close_search();
            self.refresh_bookmark();
            return;
        }
        if self.juz_mode
            && self.collection == CollectionId::Quran
            && self.active_panel == Panel::Ayahs
        {
            let entries = crate::quran::juz_entries(&self.surahs, self.current_juz);
            if let Some((surah_index, ayah_index)) =
                entries.get(self.ayah_list.selected().unwrap_or(0)).copied()
            {
                self.current_surah = surah_index;
                self.current_ayah = ayah_index;
                self.surah_list.select(Some(surah_index));
                self.ayah_list.select(Some(ayah_index));
                self.active_panel = Panel::Scripture;
                self.scripture_scroll = 0;
                self.ensure_transliteration();
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

    fn close_search(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.search_results.clear();
        self.search_list.select(None);
        self.reset_quit();
    }

    fn update_search(&mut self) {
        use crate::search::{search_collection, CollectionChapter};
        self.search_results = if self.collection == CollectionId::Quran {
            search_quran(&self.search_query, &self.surahs)
        } else {
            // Cached chapters only: unopened chapters have no rows to match
            // and search never touches the network.
            let mut chapters: Vec<CollectionChapter> = Vec::new();
            for book in &self.books {
                for ((cached_book, chapter), texts) in &self.chapter_cache {
                    if *cached_book == book.id {
                        chapters.push(CollectionChapter {
                            book_id: &book.id,
                            book_label: self.collection.book_label(&book.id, &book.name),
                            chapter: *chapter,
                            arabic: &texts.arabic,
                            english: &texts.english,
                        });
                    }
                }
            }
            search_collection(
                self.collection.key(),
                self.collection.label(),
                &chapters,
                &self.search_query,
                self.language == Language::English,
            )
        };
        self.search_list
            .select((!self.search_results.is_empty()).then_some(0));
    }

    fn copy_ayah(&mut self) {
        let Some((text, label)) = self.copy_text() else {
            return;
        };
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
            Ok(()) => self.set_status(format!("Copied {label}")),
            Err(_) => self.set_error("Copy failed — clipboard unavailable"),
        }
    }

    /// Clipboard text plus its short label for whatever the Scripture
    /// panel currently shows: one ayah/verse, or a whole chapter.
    fn copy_text(&self) -> Option<(String, String)> {
        if self.collection == CollectionId::Quran {
            let surah = self.surahs.get(self.current_surah)?;
            let ayah = surah.ayahs.get(self.current_ayah)?;
            let translation = match self.language {
                Language::Arabic | Language::Hebrew | Language::Greek => String::new(),
                Language::English => format!("\n{}", ayah.english),
            };
            return Some((
                crate::output::sanitize_terminal_text(&format!(
                    "{} {}:{}\n{}{}",
                    surah.name_transliterated, surah.number, ayah.number, ayah.arabic, translation
                )),
                format!("{}:{}", surah.number, ayah.number),
            ));
        }

        let (verse_id, arabic) = self.active_verse()?;
        // Psalm and Hadith shelves hold one unit each and name the unit;
        // every other shelf names the book.
        let heading = match self.collection {
            CollectionId::Zabur => self.unit_label().to_string(),
            _ => self.current_book_label(),
        };
        let mut text = format!("{heading} {}:{verse_id}\n{arabic}", self.current_chapter);
        if self.language == Language::English {
            if let Some(english) = self.active_verse_english(&verse_id) {
                text.push_str(&format!("\n{english}"));
            }
        }
        Some((
            crate::output::sanitize_terminal_text(&text),
            self.current_bookmark_key().unwrap_or_default(),
        ))
    }

    fn toggle_bookmark(&mut self) {
        if self.bookmark_error.is_some() {
            let message = self
                .bookmark_error
                .as_deref()
                .map(|reason| format!("Bookmarks unavailable ({reason})"))
                .unwrap_or_else(|| "Bookmarks unavailable".to_string());
            self.set_error(message);
            return;
        }
        let Some(key) = self.current_bookmark_key() else {
            self.set_error("Nothing to bookmark here");
            return;
        };
        let display = self.bookmark_display();

        let mut bookmarks = crate::bookmarks::load().unwrap_or_default();
        let (bookmarked, message) =
            if let Some(position) = bookmarks.iter().position(|marked| *marked == key) {
                bookmarks.remove(position);
                (false, format!("Removed bookmark {display}"))
            } else {
                bookmarks.push(key);
                (true, format!("Bookmarked {display}"))
            };
        match crate::bookmarks::save(&bookmarks) {
            Ok(()) => {
                self.bookmarked = bookmarked;
                self.set_status(message);
            }
            Err(error) => self.set_error(format!("Could not save bookmark ({error})")),
        }
    }

    /// Storage key for whatever the Scripture panel currently shows: Quran
    /// ayah and other-shelf verses keep verse keys; single-verse hadith
    /// chapters keep chapter keys.
    fn current_bookmark_key(&self) -> Option<String> {
        if self.collection == CollectionId::Quran {
            let surah = self.surahs.get(self.current_surah)?;
            let ayah = surah.ayahs.get(self.current_ayah)?;
            return Some(crate::bookmarks::quran_key(surah.number, ayah.number));
        }
        let book_id = self.books.get(self.current_book)?.id.clone();
        if self.collection.is_hadith() {
            return Some(crate::bookmarks::chapter_key(
                self.collection.key(),
                &book_id,
                self.current_chapter as u32,
            ));
        }
        let (verse_id, _) = self.active_verse()?;
        Some(crate::bookmarks::verse_key(
            self.collection.key(),
            &book_id,
            self.current_chapter as u32,
            &verse_id,
        ))
    }

    fn bookmark_display(&self) -> String {
        match self.collection {
            CollectionId::Quran => self
                .surahs
                .get(self.current_surah)
                .and_then(|surah| {
                    surah
                        .ayahs
                        .get(self.current_ayah)
                        .map(|ayah| format!("{}:{}", surah.number, ayah.number))
                })
                .unwrap_or_default(),
            _ => self.current_bookmark_key().unwrap_or_default(),
        }
    }

    fn refresh_bookmark(&mut self) {
        self.bookmarked = self.bookmark_error.is_none()
            && self
                .current_bookmark_key()
                .is_some_and(|key| crate::bookmarks::is_bookmarked(&key));
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status_msg = Some(crate::output::sanitize_terminal_text(&message.into()));
        self.status_error = false;
        self.status_started = Some(Instant::now());
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.status_msg = Some(crate::output::sanitize_terminal_text(&message.into()));
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
        let loaded = match data::load_quran() {
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
                    if action == crate::input::AppAction::Redraw {
                        // Full repaint, not a diff: any desync between the
                        // buffer and the real terminal (missed resize, wrapped
                        // row, dropped escape) heals here instead of haunting
                        // every later frame.
                        terminal.clear()?;
                    } else if state.apply_action(action) {
                        return Ok(());
                    }
                    dirty = true;
                }
                Event::Resize(_, _) => {
                    terminal.clear()?;
                    dirty = true;
                }
                _ => {}
            }
        } else if state.expire_transients() {
            dirty = true;
        }
    }
}
