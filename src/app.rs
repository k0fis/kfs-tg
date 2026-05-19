use crossterm::event::KeyEvent;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::keys::{self, Action, Mode};
use crate::tg;
use crate::tg::types::{Chat, Message};

#[derive(Debug, Clone)]
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
    pub client_id: i32,
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
    pub help_visible: bool,
}

impl App {
    pub fn new(config: Config, client_id: i32, event_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
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
            client_id,
            event_tx,
            help_visible: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.help_visible {
            self.help_visible = false;
            return false;
        }

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
            Action::Help => self.help_visible = true,
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
            crossterm::event::KeyCode::Enter if !self.input.is_empty() => {
                self.submit_auth();
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

    fn submit_auth(&mut self) {
        let input = self.input.clone();
        let client_id = self.client_id;
        let auth_state = self.auth_state;

        self.status = "Submitting...".to_string();
        self.input.clear();
        self.input_cursor = 0;

        tokio::spawn(async move {
            let result = match auth_state {
                AuthState::WaitPhone => tg::submit_phone(&input, client_id).await,
                AuthState::WaitCode => tg::submit_code(&input, client_id).await,
                AuthState::WaitPassword => tg::submit_password(&input, client_id).await,
                AuthState::Ready => Ok(()),
            };
            if let Err(e) = result {
                tracing::error!("Auth submit error: {e}");
            }
        });
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::AuthStatePhone => {
                self.auth_state = AuthState::WaitPhone;
                self.input.clear();
                self.input_cursor = 0;
                self.status = "Enter phone number (with +country code):".to_string();
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
                self.status = format!("{} chats loaded", self.chats.len());
            }
            AppEvent::MessagesLoaded(msgs) => {
                self.messages = msgs;
                if !self.messages.is_empty() {
                    self.msg_cursor = self.messages.len() - 1;
                }
            }
            AppEvent::NewMessage(msg) => {
                if let Some(chat) = self.chats.get(self.chat_cursor)
                    && msg.chat_id == chat.id
                {
                    self.messages.push(msg);
                    self.msg_cursor = self.messages.len().saturating_sub(1);
                }
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
        if let Some(chat) = self.chats.get(self.chat_cursor) {
            let chat_id = chat.id;
            let client_id = self.client_id;
            let tx = self.event_tx.clone();
            self.panel = Panel::Messages;
            self.messages.clear();
            self.msg_cursor = 0;
            self.status = "Loading messages...".to_string();

            tokio::spawn(async move {
                tg::load_chat_messages(chat_id, client_id, &tx).await;
            });
        }
    }

    fn send_message(&mut self) {
        if !self.input.is_empty()
            && let Some(chat) = self.chats.get(self.chat_cursor)
        {
            let chat_id = chat.id;
            let client_id = self.client_id;
            let text = self.input.clone();

            self.input.clear();
            self.input_cursor = 0;
            self.mode = Mode::Normal;

            tokio::spawn(async move {
                if let Err(e) = tg::send_text_message(chat_id, &text, client_id).await {
                    tracing::error!("Send message error: {e}");
                }
            });
        }
    }
}
