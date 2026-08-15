use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::InputMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,
    NextTab,
    PreviousTab,
    MoveUp,
    MoveDown,
    First,
    Last,
    TogglePause,
    Refresh,
    ToggleHelp,
    CloseOverlay,
    OpenDetails,
    StartSearch,
    FinishSearch,
    ClearSearch,
    SearchChar(char),
    SearchBackspace,
    ExportReport,
    None,
}

pub fn from_key(key: KeyEvent, input_mode: InputMode) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Action::Quit;
    }

    if input_mode == InputMode::Search {
        return match key.code {
            KeyCode::Esc => Action::ClearSearch,
            KeyCode::Enter => Action::FinishSearch,
            KeyCode::Backspace => Action::SearchBackspace,
            KeyCode::Char(character) => Action::SearchChar(character),
            _ => Action::None,
        };
    }

    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => Action::NextTab,
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => Action::PreviousTab,
        KeyCode::Up | KeyCode::Char('k') => Action::MoveUp,
        KeyCode::Down | KeyCode::Char('j') => Action::MoveDown,
        KeyCode::Home | KeyCode::Char('g') => Action::First,
        KeyCode::End | KeyCode::Char('G') => Action::Last,
        KeyCode::Char(' ') => Action::TogglePause,
        KeyCode::Char('r') => Action::Refresh,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Esc => Action::CloseOverlay,
        KeyCode::Enter => Action::OpenDetails,
        KeyCode::Char('/') => Action::StartSearch,
        KeyCode::Char('e') => Action::ExportReport,
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_navigation_and_global_quit() {
        assert_eq!(
            from_key(
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
                InputMode::Normal
            ),
            Action::MoveDown
        );
        assert_eq!(
            from_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                InputMode::Search
            ),
            Action::Quit
        );
    }

    #[test]
    fn search_mode_routes_characters_to_search() {
        assert_eq!(
            from_key(
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
                InputMode::Search
            ),
            Action::SearchChar('q')
        );
        assert_eq!(
            from_key(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                InputMode::Search
            ),
            Action::ClearSearch
        );
    }
}
