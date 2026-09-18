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
    ToggleHelp,
    QuitOne,
    QuitConfirm,
    Noop,
}

pub fn handle_key(key: KeyEvent, state: &AppState) -> AppAction {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return AppAction::QuitConfirm;
    }

    if state.show_help {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                AppAction::ToggleHelp
            }
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
        KeyCode::Char('?') => AppAction::ToggleHelp,
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
