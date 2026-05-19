use crossterm::event::KeyEvent;

use crate::config::Config;
use crate::keys::{self, Action, Mode};
use crate::tg::types::{Chat, Message};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppEvent {
    AuthStatePhone,
    AuthStateCode,
    AuthStatePassword,
    AuthStateReady,
    ChatsLoaded(Vec<Chat>),
    MessagesLoaded(Vec<Message>),
    NewMessage(Message),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Login,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    ChatList,
    Messages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthState {
    WaitPhone,
    WaitCode,
    WaitPassword,
    Ready,
}

pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub panel: Panel,
    pub mode: Mode,
    pub auth_state: AuthState,
    pub input: String,
    pub input_cursor: usize,
    pub chats: Vec<Chat>,
    pub messages: Vec<Message>,
    pub chat_cursor: usize,
    pub msg_cursor: usize,
    pub status: String,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            screen: Screen::Login,
            panel: Panel::ChatList,
            mode: Mode::Normal,
            auth_state: AuthState::WaitPhone,
            input: String::new(),
            input_cursor: 0,
            chats: Vec::new(),
            messages: Vec::new(),
            chat_cursor: 0,
            msg_cursor: 0,
            status: "Connecting...".to_string(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.screen == Screen::Login {
            return self.handle_login_key(key);
        }

        let action = keys::map_key(key, self.mode);
        match action {
            Action::Quit => return true,
            Action::MoveDown => self.move_down(),
            Action::MoveUp => self.move_up(),
            Action::MoveLeft => self.panel = Panel::ChatList,
            Action::MoveRight => self.panel = Panel::Messages,
            Action::Enter => self.select_chat(),
            Action::EnterInsert => {
                self.mode = Mode::Insert;
                self.panel = Panel::Messages;
            }
            Action::ExitInsert => self.mode = Mode::Normal,
            Action::SendMessage => self.send_message(),
            Action::Char(c) if self.mode == Mode::Insert => {
                self.input.insert(self.input_cursor, c);
                self.input_cursor += c.len_utf8();
            }
            Action::Backspace if self.mode == Mode::Insert && self.input_cursor > 0 => {
                let prev = self.input[..self.input_cursor]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                self.input_cursor -= prev;
                self.input.remove(self.input_cursor);
            }
            Action::GoTop => match self.panel {
                Panel::ChatList => self.chat_cursor = 0,
                Panel::Messages => self.msg_cursor = 0,
            },
            Action::GoBottom => match self.panel {
                Panel::ChatList if !self.chats.is_empty() => {
                    self.chat_cursor = self.chats.len() - 1;
                }
                Panel::Messages if !self.messages.is_empty() => {
                    self.msg_cursor = self.messages.len() - 1;
                }
                _ => {}
            },
            _ => {}
        }
        false
    }

    fn handle_login_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            crossterm::event::KeyCode::Char(c) => {
                self.input.insert(self.input_cursor, c);
                self.input_cursor += c.len_utf8();
            }
            crossterm::event::KeyCode::Backspace if self.input_cursor > 0 => {
                let prev = self.input[..self.input_cursor]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                self.input_cursor -= prev;
                self.input.remove(self.input_cursor);
            }
            crossterm::event::KeyCode::Enter => {
                // Submit input to TDLib (handled via channel)
                self.status = "Submitting...".to_string();
            }
            crossterm::event::KeyCode::Left if self.input_cursor > 0 => {
                let prev = self.input[..self.input_cursor]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                self.input_cursor -= prev;
            }
            crossterm::event::KeyCode::Right if self.input_cursor < self.input.len() => {
                let next = self.input[self.input_cursor..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                self.input_cursor += next;
            }
            _ => {}
        }
        false
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::AuthStatePhone => {
                self.auth_state = AuthState::WaitPhone;
                self.input.clear();
                self.input_cursor = 0;
                self.status = "Enter phone number:".to_string();
            }
            AppEvent::AuthStateCode => {
                self.auth_state = AuthState::WaitCode;
                self.input.clear();
                self.input_cursor = 0;
                self.status = "Enter verification code:".to_string();
            }
            AppEvent::AuthStatePassword => {
                self.auth_state = AuthState::WaitPassword;
                self.input.clear();
                self.input_cursor = 0;
                self.status = "Enter 2FA password:".to_string();
            }
            AppEvent::AuthStateReady => {
                self.auth_state = AuthState::Ready;
                self.screen = Screen::Main;
                self.status = "Connected".to_string();
            }
            AppEvent::ChatsLoaded(chats) => {
                self.chats = chats;
            }
            AppEvent::MessagesLoaded(msgs) => {
                self.messages = msgs;
                if !self.messages.is_empty() {
                    self.msg_cursor = self.messages.len() - 1;
                }
            }
            AppEvent::NewMessage(msg) => {
                self.messages.push(msg);
                self.msg_cursor = self.messages.len().saturating_sub(1);
            }
            AppEvent::Error(e) => {
                self.status = format!("Error: {e}");
            }
        }
    }

    fn move_down(&mut self) {
        match self.panel {
            Panel::ChatList if !self.chats.is_empty() => {
                self.chat_cursor = (self.chat_cursor + 1).min(self.chats.len() - 1);
            }
            Panel::Messages if !self.messages.is_empty() => {
                self.msg_cursor = (self.msg_cursor + 1).min(self.messages.len() - 1);
            }
            _ => {}
        }
    }

    fn move_up(&mut self) {
        match self.panel {
            Panel::ChatList => {
                self.chat_cursor = self.chat_cursor.saturating_sub(1);
            }
            Panel::Messages => {
                self.msg_cursor = self.msg_cursor.saturating_sub(1);
            }
        }
    }

    fn select_chat(&mut self) {
        if !self.chats.is_empty() {
            self.panel = Panel::Messages;
            // TODO: trigger message load for selected chat
        }
    }

    fn send_message(&mut self) {
        if !self.input.is_empty() {
            // TODO: send via TDLib
            self.input.clear();
            self.input_cursor = 0;
            self.mode = Mode::Normal;
        }
    }
}
