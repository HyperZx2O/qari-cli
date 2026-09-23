use crate::app::AppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    PanelLeft,
    PanelRight,
    SelectUp,
    SelectDown,
    Select,
    OpenSearch,
    CloseSearch,
    SearchInput(char),
    SearchBackspace,
    CopyAyah,
    ToggleBookmark,
    CycleTheme,
    CycleLanguage,
    CycleTransliteration,
    ToggleJuzMode,
    ToggleHelp,
    ToggleVerse,
    QuitOne,
    QuitConfirm,
    Redraw,
    Noop,
}

pub fn handle_key(key: KeyEvent, state: &AppState) -> AppAction {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return AppAction::QuitConfirm;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        return AppAction::Redraw;
    }

    if state.show_help {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                AppAction::ToggleHelp
            }
            KeyCode::Up | KeyCode::Char('k') => AppAction::SelectUp,
            KeyCode::Down | KeyCode::Char('j') => AppAction::SelectDown,
            _ => AppAction::Noop,
        };
    }

    if state.search_mode {
        return match key.code {
            KeyCode::Esc => AppAction::CloseSearch,
            KeyCode::Backspace => AppAction::SearchBackspace,
            KeyCode::Up => AppAction::SelectUp,
            KeyCode::Down => AppAction::SelectDown,
            KeyCode::Enter => AppAction::Select,
            KeyCode::Char(character)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                ) =>
            {
                AppAction::SearchInput(character)
            }
            _ => AppAction::Noop,
        };
    }

    if state.show_verse {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('f') | KeyCode::Char('q') | KeyCode::Enter => {
                AppAction::ToggleVerse
            }
            // j/k scroll the verse modal; the verse cursor stays put.
            KeyCode::Up | KeyCode::Char('k') => AppAction::SelectUp,
            KeyCode::Down | KeyCode::Char('j') => AppAction::SelectDown,
            _ => AppAction::Noop,
        };
    }

    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return AppAction::Noop;
    }

    match key.code {
        KeyCode::Left | KeyCode::Char('h') => AppAction::PanelLeft,
        KeyCode::Right | KeyCode::Char('l') => AppAction::PanelRight,
        KeyCode::Down | KeyCode::Char('j') => AppAction::SelectDown,
        KeyCode::Up | KeyCode::Char('k') => AppAction::SelectUp,
        KeyCode::Enter => AppAction::Select,
        KeyCode::Char('/') => AppAction::OpenSearch,
        KeyCode::Esc => AppAction::CloseSearch,
        KeyCode::Char('y') | KeyCode::Char('c') => AppAction::CopyAyah,
        KeyCode::Char('b') => AppAction::ToggleBookmark,
        KeyCode::Char('t') => AppAction::CycleTheme,
        KeyCode::Char('v') => AppAction::CycleLanguage,
        KeyCode::Char('i') => AppAction::CycleTransliteration,
        KeyCode::Char('J') => AppAction::ToggleJuzMode,
        KeyCode::Char('?') => AppAction::ToggleHelp,
        KeyCode::Char('f') => AppAction::ToggleVerse,
        KeyCode::Char('q') => {
            if state.quit_count == 1
                && state
                    .quit_started
                    .is_some_and(|started| started.elapsed() <= Duration::from_millis(500))
            {
                AppAction::QuitConfirm
            } else {
                AppAction::QuitOne
            }
        }
        _ => AppAction::Noop,
    }
}
