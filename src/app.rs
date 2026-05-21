use crossterm::event::KeyEvent;
use ratatui::widgets::ListState;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::keys::{self, Action, Mode};
use crate::tg;
use crate::tg::types::{Chat, ChatKind, Message};

#[derive(Debug, Clone)]
pub enum AppEvent {
    AuthStatePhone,
    AuthStateCode,
    AuthStatePassword,
    AuthStateReady,
    ChatsLoaded(Vec<Chat>),
    FoldersLoaded(Vec<(i32, String)>),
    MessagesLoaded(Vec<Message>),
    NewMessage(Message),
    MessageEdited(i64, i64, String),
    MessagesDeleted(i64, Vec<i64>),
    OlderMessagesLoaded(Vec<Message>),
    ChatUnreadCount(i64, i32),
    UserStatus(i64, String),
    ChatAction(i64, String),
    BotCommandsLoaded(Vec<(String, String)>),
    PublicChatOpened(Chat),
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
    pub folders: Vec<(i32, String)>,
    pub active_folder: Option<i32>,
    pub chat_cursor: usize,
    pub msg_cursor: usize,
    pub status: String,
    pub client_id: i32,
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
    pub help_visible: bool,
    pub bot_commands: Vec<(String, String)>,
    pub cmd_cursor: usize,
    pub cmd_visible: bool,
    pub cmd_filter: String,
    pub reply_to: Option<(i64, String)>,
    pub edit_msg: Option<i64>,
    pub confirm_delete: Option<i64>,
    pub forward_msg: Option<i64>,
    pub forward_cursor: usize,
    pub search_query: String,
    pub search_active: bool,
    pub msg_search_query: String,
    pub msg_search_active: bool,
    pub open_chat_active: bool,
    pub open_chat_query: String,
    pub typing_status: String,
    pub loading_older: bool,
    pub msg_list_state: ListState,
    pub unread_from_id: Option<i64>,
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
            folders: Vec::new(),
            active_folder: None,
            chat_cursor: 0,
            msg_cursor: 0,
            status: "Connecting...".to_string(),
            client_id,
            event_tx,
            help_visible: false,
            bot_commands: Vec::new(),
            cmd_cursor: 0,
            cmd_visible: false,
            cmd_filter: String::new(),
            reply_to: None,
            edit_msg: None,
            confirm_delete: None,
            forward_msg: None,
            forward_cursor: 0,
            search_query: String::new(),
            search_active: false,
            msg_search_query: String::new(),
            msg_search_active: false,
            open_chat_active: false,
            open_chat_query: String::new(),
            typing_status: String::new(),
            loading_older: false,
            msg_list_state: ListState::default(),
            unread_from_id: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.help_visible {
            self.help_visible = false;
            return false;
        }

        if let Some(msg_id) = self.confirm_delete {
            return self.handle_delete_confirm(key, msg_id);
        }

        if self.forward_msg.is_some() {
            return self.handle_forward_key(key);
        }

        if self.cmd_visible {
            return self.handle_cmd_key(key);
        }

        if self.search_active {
            return self.handle_search_key(key);
        }

        if self.msg_search_active {
            return self.handle_msg_search_key(key);
        }

        if self.open_chat_active {
            return self.handle_open_chat_key(key);
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
            Action::ExitInsert => {
                self.mode = Mode::Normal;
                self.reply_to = None;
                self.edit_msg = None;
            }
            Action::Help => self.help_visible = true,
            Action::Reply => self.start_reply(),
            Action::Forward => self.start_forward(),
            Action::EditMsg => self.start_edit(),
            Action::OpenMedia => self.open_media(),
            Action::Delete => self.start_delete(),
            Action::Search => self.trigger_bot_commands(),
            Action::Refresh => self.refresh_chats(),
            Action::SearchChats => {
                self.search_active = true;
                self.search_query.clear();
                self.panel = Panel::ChatList;
            }
            Action::SearchMessages => {
                self.msg_search_active = true;
                self.msg_search_query.clear();
                self.panel = Panel::Messages;
            }
            Action::OpenChat => {
                self.open_chat_active = true;
                self.open_chat_query.clear();
            }
            Action::SendMessage => self.send_message(),
            Action::NewLine if self.mode == Mode::Insert => {
                self.input.insert(self.input_cursor, '\n');
                self.input_cursor += 1;
            }
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
            Action::CursorLeft if self.input_cursor > 0 => {
                let prev = self.input[..self.input_cursor]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                self.input_cursor -= prev;
            }
            Action::CursorRight if self.input_cursor < self.input.len() => {
                let next = self.input[self.input_cursor..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                self.input_cursor += next;
            }
            Action::CursorWordLeft if self.input_cursor > 0 => {
                let before = &self.input[..self.input_cursor];
                let chars: Vec<char> = before.chars().collect();
                let mut i = chars.len();
                while i > 0 && !chars[i - 1].is_alphanumeric() {
                    i -= 1;
                }
                while i > 0 && chars[i - 1].is_alphanumeric() {
                    i -= 1;
                }
                self.input_cursor = chars[..i].iter().map(|c| c.len_utf8()).sum();
            }
            Action::CursorWordRight if self.input_cursor < self.input.len() => {
                let after = &self.input[self.input_cursor..];
                let chars: Vec<char> = after.chars().collect();
                let mut i = 0;
                while i < chars.len() && !chars[i].is_alphanumeric() {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_alphanumeric() {
                    i += 1;
                }
                self.input_cursor += chars[..i].iter().map(|c| c.len_utf8()).sum::<usize>();
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
            Action::PageDown => match self.panel {
                Panel::ChatList if !self.chats.is_empty() => {
                    self.chat_cursor = (self.chat_cursor + 10).min(self.chats.len() - 1);
                }
                Panel::Messages if !self.messages.is_empty() => {
                    self.msg_cursor = (self.msg_cursor + 10).min(self.messages.len() - 1);
                }
                _ => {}
            },
            Action::PageUp => match self.panel {
                Panel::ChatList => {
                    self.chat_cursor = self.chat_cursor.saturating_sub(10);
                }
                Panel::Messages => {
                    self.msg_cursor = self.msg_cursor.saturating_sub(10);
                }
            },
            Action::SwitchFolder(n) => self.switch_folder(n),
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

    pub fn handle_paste(&mut self, text: &str) {
        if self.mode == Mode::Insert {
            let clean = text.replace("\r\n", "\n").replace('\r', "\n");
            self.input.insert_str(self.input_cursor, &clean);
            self.input_cursor += clean.len();
        }
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
                self.chat_cursor = 0;
                self.status = format!("{} chats loaded", self.chats.len());
            }
            AppEvent::FoldersLoaded(folders) => {
                self.folders = folders;
            }
            AppEvent::MessagesLoaded(msgs) => {
                self.messages = msgs;
                if !self.messages.is_empty() {
                    self.msg_cursor = self.messages.len() - 1;
                }
            }
            AppEvent::NewMessage(msg) => {
                let current_chat_id = self.chats.get(self.chat_cursor).map(|c| c.id);
                if current_chat_id == Some(msg.chat_id) {
                    self.messages.push(msg);
                    self.msg_cursor = self.messages.len().saturating_sub(1);
                } else if !msg.is_outgoing && self.config.ui.notifications {
                    let chat_title = self
                        .chats
                        .iter()
                        .find(|c| c.id == msg.chat_id)
                        .map(|c| c.title.as_str())
                        .unwrap_or("New message");
                    let body = if msg.text.len() > 80 {
                        format!("{}...", &msg.text[..80])
                    } else {
                        msg.text.clone()
                    };
                    let _ = notify_rust::Notification::new()
                        .summary(chat_title)
                        .body(&body)
                        .appname("kfs-tg")
                        .show();
                }
            }
            AppEvent::MessageEdited(chat_id, msg_id, new_text) => {
                if let Some(chat) = self.chats.get(self.chat_cursor)
                    && chat_id == chat.id
                    && let Some(msg) = self.messages.iter_mut().find(|m| m.id == msg_id)
                {
                    msg.text = new_text;
                }
            }
            AppEvent::MessagesDeleted(chat_id, msg_ids) => {
                if let Some(chat) = self.chats.get(self.chat_cursor)
                    && chat_id == chat.id
                {
                    self.messages.retain(|m| !msg_ids.contains(&m.id));
                    self.msg_cursor = self.msg_cursor.min(self.messages.len().saturating_sub(1));
                }
            }
            AppEvent::OlderMessagesLoaded(older) => {
                self.loading_older = false;
                if !older.is_empty() {
                    let count = older.len();
                    let mut combined = older;
                    combined.append(&mut self.messages);
                    combined.sort_by_key(|m| m.timestamp);
                    self.messages = combined;
                    self.msg_cursor += count;
                }
            }
            AppEvent::ChatUnreadCount(chat_id, count) => {
                if let Some(chat) = self.chats.iter_mut().find(|c| c.id == chat_id) {
                    chat.unread_count = count;
                }
            }
            AppEvent::UserStatus(_user_id, _status) => {
                // Could show in chat list, for now just stored for future use
            }
            AppEvent::ChatAction(chat_id, action) => {
                if let Some(chat) = self.chats.get(self.chat_cursor)
                    && chat_id == chat.id
                {
                    self.typing_status = action;
                }
            }
            AppEvent::BotCommandsLoaded(cmds) => {
                if cmds.is_empty() {
                    self.input = "/".to_string();
                    self.input_cursor = 1;
                    self.mode = Mode::Insert;
                    self.panel = Panel::Messages;
                } else {
                    self.status = format!("{} commands", cmds.len());
                    self.bot_commands = cmds;
                    self.cmd_cursor = 0;
                    self.cmd_filter.clear();
                    self.cmd_visible = true;
                }
            }
            AppEvent::PublicChatOpened(chat) => {
                if !self.chats.iter().any(|c| c.id == chat.id) {
                    self.chats.insert(0, chat.clone());
                    self.chat_cursor = 0;
                } else {
                    self.chat_cursor = self.chats.iter().position(|c| c.id == chat.id).unwrap_or(0);
                }
                self.select_chat();
                self.status = format!("Opened: {}", chat.title);
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
                if self.msg_cursor == 0 && !self.loading_older {
                    self.load_older_messages();
                } else {
                    self.msg_cursor = self.msg_cursor.saturating_sub(1);
                }
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
            self.msg_list_state = ListState::default();
            self.loading_older = false;
            self.status = "Loading messages...".to_string();
            self.unread_from_id = if chat.unread_count > 0 {
                Some(chat.last_read_inbox_message_id)
            } else {
                None
            };

            tokio::spawn(async move {
                tg::load_chat_messages(chat_id, client_id, &tx).await;
            });
        }
    }

    fn load_older_messages(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let oldest_id = self.messages[0].id;
        if let Some(chat) = self.chats.get(self.chat_cursor) {
            let chat_id = chat.id;
            let client_id = self.client_id;
            let tx = self.event_tx.clone();
            self.loading_older = true;
            tokio::spawn(async move {
                tg::load_older_messages(chat_id, oldest_id, client_id, &tx).await;
            });
        }
    }

    fn refresh_chats(&mut self) {
        let client_id = self.client_id;
        let tx = self.event_tx.clone();
        self.status = "Refreshing...".to_string();
        tokio::spawn(async move {
            tg::refresh_chats(client_id, &tx).await;
        });
    }

    fn switch_folder(&mut self, n: u8) {
        if n == 0 {
            self.active_folder = None;
            self.chat_cursor = 0;
            let client_id = self.client_id;
            let tx = self.event_tx.clone();
            self.status = "All chats...".to_string();
            tokio::spawn(async move {
                tg::load_chats_for_folder(None, client_id, &tx).await;
            });
        } else {
            let idx = (n - 1) as usize;
            if let Some((folder_id, folder_name)) = self.folders.get(idx) {
                let fid = *folder_id;
                let fname = folder_name.clone();
                self.active_folder = Some(fid);
                self.chat_cursor = 0;
                let client_id = self.client_id;
                let tx = self.event_tx.clone();
                self.status = format!("Folder: {fname}...");
                tokio::spawn(async move {
                    tg::load_chats_for_folder(Some(fid), client_id, &tx).await;
                });
            }
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

            if let Some(msg_id) = self.edit_msg.take() {
                tokio::spawn(async move {
                    if let Err(e) = tg::edit_message_text(chat_id, msg_id, &text, client_id).await {
                        tracing::error!("Edit message error: {e}");
                    }
                });
            } else {
                let reply_to_id = self.reply_to.as_ref().map(|(id, _)| *id);
                self.reply_to = None;
                tokio::spawn(async move {
                    if let Err(e) =
                        tg::send_text_message(chat_id, &text, reply_to_id, client_id).await
                    {
                        tracing::error!("Send message error: {e}");
                    }
                });
            }
        }
    }

    fn start_reply(&mut self) {
        if self.panel == Panel::Messages
            && let Some(msg) = self.messages.get(self.msg_cursor)
        {
            let preview = if msg.text.len() > 30 {
                format!("{}...", &msg.text[..30])
            } else {
                msg.text.clone()
            };
            self.reply_to = Some((msg.id, preview));
            self.mode = Mode::Insert;
            self.panel = Panel::Messages;
        }
    }

    fn start_edit(&mut self) {
        if self.panel == Panel::Messages
            && let Some(msg) = self.messages.get(self.msg_cursor)
            && msg.is_outgoing
        {
            self.edit_msg = Some(msg.id);
            self.input = msg.text.clone();
            self.input_cursor = self.input.len();
            self.mode = Mode::Insert;
            self.panel = Panel::Messages;
        }
    }

    fn open_media(&mut self) {
        if self.panel == Panel::Messages
            && let Some(msg) = self.messages.get(self.msg_cursor)
            && let Some(file_id) = msg.file_id
        {
            let client_id = self.client_id;
            self.status = "Downloading...".to_string();
            tokio::spawn(async move {
                if let Err(e) = tg::download_and_open(file_id, client_id).await {
                    tracing::error!("Open media error: {e}");
                }
            });
        }
    }

    fn start_delete(&mut self) {
        if self.panel == Panel::Messages
            && let Some(msg) = self.messages.get(self.msg_cursor)
            && msg.is_outgoing
        {
            self.confirm_delete = Some(msg.id);
            self.status = "Delete this message? (y/n)".to_string();
        }
    }

    fn handle_delete_confirm(&mut self, key: KeyEvent, msg_id: i64) -> bool {
        self.confirm_delete = None;
        if key.code == crossterm::event::KeyCode::Char('y') {
            if let Some(chat) = self.chats.get(self.chat_cursor) {
                let chat_id = chat.id;
                let client_id = self.client_id;
                tokio::spawn(async move {
                    let _ = tdlib_rs::functions::delete_messages(
                        chat_id,
                        vec![msg_id],
                        true,
                        client_id,
                    )
                    .await;
                });
                self.messages.retain(|m| m.id != msg_id);
                self.msg_cursor = self.msg_cursor.min(self.messages.len().saturating_sub(1));
                self.status = "Message deleted".to_string();
            }
        } else {
            self.status = "Delete cancelled".to_string();
        }
        false
    }

    fn handle_cmd_key(&mut self, key: KeyEvent) -> bool {
        let filtered = self.filtered_bot_commands();
        match key.code {
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down
                if !filtered.is_empty() =>
            {
                self.cmd_cursor = (self.cmd_cursor + 1).min(filtered.len() - 1);
            }
            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                self.cmd_cursor = self.cmd_cursor.saturating_sub(1);
            }
            crossterm::event::KeyCode::Enter => {
                let filtered = self.filtered_bot_commands();
                if let Some((cmd, _)) = filtered.get(self.cmd_cursor) {
                    self.input = format!("/{cmd}");
                    self.input_cursor = self.input.len();
                    self.mode = Mode::Insert;
                    self.panel = Panel::Messages;
                }
                self.cmd_visible = false;
                self.cmd_filter.clear();
            }
            crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q') => {
                self.cmd_visible = false;
                self.cmd_filter.clear();
            }
            crossterm::event::KeyCode::Backspace => {
                self.cmd_filter.pop();
                self.cmd_cursor = 0;
            }
            crossterm::event::KeyCode::Char(c)
                if c.is_alphanumeric() || c == '_' =>
            {
                self.cmd_filter.push(c);
                self.cmd_cursor = 0;
            }
            _ => {}
        }
        false
    }

    pub fn filtered_bot_commands(&self) -> Vec<(String, String)> {
        if self.cmd_filter.is_empty() {
            return self.bot_commands.clone();
        }
        let q = self.cmd_filter.to_lowercase();
        self.bot_commands
            .iter()
            .filter(|(cmd, _)| cmd.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    fn start_forward(&mut self) {
        if self.panel == Panel::Messages
            && let Some(msg) = self.messages.get(self.msg_cursor)
        {
            self.forward_msg = Some(msg.id);
            self.forward_cursor = 0;
            self.status = "Forward to: j/k navigate, Enter select, Esc cancel".to_string();
        }
    }

    fn handle_forward_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q') => {
                self.forward_msg = None;
                self.status = "Forward cancelled".to_string();
            }
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down
                if !self.chats.is_empty() =>
            {
                self.forward_cursor = (self.forward_cursor + 1).min(self.chats.len() - 1);
            }
            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                self.forward_cursor = self.forward_cursor.saturating_sub(1);
            }
            crossterm::event::KeyCode::Enter => {
                if let Some(msg_id) = self.forward_msg.take()
                    && let (Some(from_chat), Some(to_chat)) = (
                        self.chats.get(self.chat_cursor),
                        self.chats.get(self.forward_cursor),
                    )
                {
                    let from_id = from_chat.id;
                    let to_id = to_chat.id;
                    let client_id = self.client_id;
                    let to_title = to_chat.title.clone();
                    self.status = format!("Forwarded to {to_title}");
                    tokio::spawn(async move {
                        let _ = tdlib_rs::functions::forward_messages(
                            to_id,
                            None,
                            from_id,
                            vec![msg_id],
                            None,
                            false,
                            false,
                            client_id,
                        )
                        .await;
                    });
                }
            }
            _ => {}
        }
        false
    }

    fn trigger_bot_commands(&mut self) {
        if let Some(chat) = self.chats.get(self.chat_cursor) {
            let kind = chat.kind;
            if kind == ChatKind::Channel {
                self.input = "/".to_string();
                self.input_cursor = 1;
                self.mode = Mode::Insert;
                self.panel = Panel::Messages;
                return;
            }
            let chat_id = chat.id;
            let client_id = self.client_id;
            let tx = self.event_tx.clone();
            self.status = format!("Loading commands ({kind:?})...");
            tokio::spawn(async move {
                let cmds = tg::get_bot_commands(chat_id, kind, client_id).await;
                let _ = tx.send(AppEvent::BotCommandsLoaded(cmds));
            });
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            crossterm::event::KeyCode::Esc => {
                self.search_active = false;
                self.search_query.clear();
            }
            crossterm::event::KeyCode::Enter => {
                self.search_active = false;
                self.select_chat();
            }
            crossterm::event::KeyCode::Backspace => {
                self.search_query.pop();
                self.snap_cursor_to_filtered();
            }
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                self.search_move_down();
            }
            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                self.search_move_up();
            }
            crossterm::event::KeyCode::Char(c) => {
                self.search_query.push(c);
                self.snap_cursor_to_filtered();
            }
            _ => {}
        }
        false
    }

    pub fn filtered_chat_indices(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            return (0..self.chats.len()).collect();
        }
        let q = self.search_query.to_lowercase();
        self.chats
            .iter()
            .enumerate()
            .filter(|(_, c)| c.title.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    fn snap_cursor_to_filtered(&mut self) {
        let indices = self.filtered_chat_indices();
        if indices.is_empty() {
            return;
        }
        if !indices.contains(&self.chat_cursor) {
            self.chat_cursor = indices[0];
        }
    }

    fn search_move_down(&mut self) {
        let indices = self.filtered_chat_indices();
        if indices.is_empty() {
            return;
        }
        if let Some(pos) = indices.iter().position(|&i| i == self.chat_cursor) {
            if pos + 1 < indices.len() {
                self.chat_cursor = indices[pos + 1];
            }
        } else {
            self.chat_cursor = indices[0];
        }
    }

    fn search_move_up(&mut self) {
        let indices = self.filtered_chat_indices();
        if indices.is_empty() {
            return;
        }
        if let Some(pos) = indices.iter().position(|&i| i == self.chat_cursor) {
            if pos > 0 {
                self.chat_cursor = indices[pos - 1];
            }
        } else {
            self.chat_cursor = indices[0];
        }
    }

    fn handle_msg_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            crossterm::event::KeyCode::Esc => {
                self.msg_search_active = false;
                self.msg_search_query.clear();
            }
            crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char('n') => {
                self.msg_search_next();
            }
            crossterm::event::KeyCode::Char('N') => {
                self.msg_search_prev();
            }
            crossterm::event::KeyCode::Backspace => {
                self.msg_search_query.pop();
                self.snap_msg_to_search();
            }
            crossterm::event::KeyCode::Char(c) => {
                self.msg_search_query.push(c);
                self.snap_msg_to_search();
            }
            _ => {}
        }
        false
    }

    fn handle_open_chat_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            crossterm::event::KeyCode::Esc => {
                self.open_chat_active = false;
                self.open_chat_query.clear();
            }
            crossterm::event::KeyCode::Enter => {
                let username = self
                    .open_chat_query
                    .trim()
                    .trim_start_matches('@')
                    .to_string();
                self.open_chat_active = false;
                self.open_chat_query.clear();
                if !username.is_empty() {
                    self.status = format!("Searching @{username}...");
                    let client_id = self.client_id;
                    let tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        tg::open_public_chat(&username, client_id, &tx).await;
                    });
                }
            }
            crossterm::event::KeyCode::Backspace => {
                self.open_chat_query.pop();
            }
            crossterm::event::KeyCode::Char(c) => {
                self.open_chat_query.push(c);
            }
            _ => {}
        }
        false
    }

    pub fn msg_search_matches(&self) -> Vec<usize> {
        if self.msg_search_query.is_empty() {
            return Vec::new();
        }
        let q = self.msg_search_query.to_lowercase();
        self.messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.text.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    fn snap_msg_to_search(&mut self) {
        let matches = self.msg_search_matches();
        if matches.is_empty() {
            return;
        }
        if let Some(&first_after) = matches.iter().find(|&&i| i >= self.msg_cursor) {
            self.msg_cursor = first_after;
        } else {
            self.msg_cursor = matches[0];
        }
    }

    fn msg_search_next(&mut self) {
        let matches = self.msg_search_matches();
        if matches.is_empty() {
            return;
        }
        if let Some(&next) = matches.iter().find(|&&i| i > self.msg_cursor) {
            self.msg_cursor = next;
        } else {
            self.msg_cursor = matches[0];
        }
    }

    fn msg_search_prev(&mut self) {
        let matches = self.msg_search_matches();
        if matches.is_empty() {
            return;
        }
        if let Some(&prev) = matches.iter().rev().find(|&&i| i < self.msg_cursor) {
            self.msg_cursor = prev;
        } else {
            self.msg_cursor = *matches.last().unwrap();
        }
    }
}
