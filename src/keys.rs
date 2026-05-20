use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Enter,
    EnterInsert,
    ExitInsert,
    SendMessage,
    NewLine,
    Search,
    Reply,
    Forward,
    Delete,
    SearchChats,
    GoTop,
    GoBottom,
    Refresh,
    Help,
    Char(char),
    Backspace,
    None,
}

pub fn map_key(key: KeyEvent, mode: Mode) -> Action {
    match mode {
        Mode::Normal => map_normal(key),
        Mode::Insert => map_insert(key),
    }
}

fn map_normal(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('h') | KeyCode::Left => Action::MoveLeft,
        KeyCode::Char('l') | KeyCode::Right => Action::MoveRight,
        KeyCode::Enter => Action::Enter,
        KeyCode::Char('i') => Action::EnterInsert,
        KeyCode::Char('/') => Action::Search,
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Refresh,
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::SearchChats,
        KeyCode::Char('r') => Action::Reply,
        KeyCode::Char('f') => Action::Forward,
        KeyCode::Char('d') => Action::Delete,
        KeyCode::Char('g') => Action::GoTop,
        KeyCode::Char('G') => Action::GoBottom,
        KeyCode::Char('?') => Action::Help,
        _ => Action::None,
    }
}

fn map_insert(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::ExitInsert,
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => Action::NewLine,
        KeyCode::Enter => Action::SendMessage,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ExitInsert,
        KeyCode::Char(c) => Action::Char(c),
        KeyCode::Backspace => Action::Backspace,
        _ => Action::None,
    }
}
