pub mod types;

use tokio::sync::mpsc;

use crate::app::AppEvent;
use crate::config::Config;
use crate::tg::types::{Chat, ChatKind, Message};

pub async fn run(
    client_id: i32,
    config: Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> anyhow::Result<()> {
    // Spawn blocking receiver thread
    let tx_recv = tx.clone();
    let cfg = config.clone();
    tokio::task::spawn_blocking(move || {
        receiver_loop(client_id, &cfg, &tx_recv);
    });

    // Suppress TDLib internal logging
    tdlib_rs::functions::set_log_verbosity_level(0, client_id)
        .await
        .ok();

    // Keep task alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}

fn receiver_loop(client_id: i32, config: &Config, tx: &mpsc::UnboundedSender<AppEvent>) {
    loop {
        if let Some((update, _cid)) = tdlib_rs::receive() {
            handle_update(update, client_id, config, tx);
        }
    }
}

fn handle_update(
    update: tdlib_rs::enums::Update,
    client_id: i32,
    config: &Config,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    use tdlib_rs::enums::Update;

    match update {
        Update::AuthorizationState(state) => {
            handle_auth_state(state.authorization_state, client_id, config, tx);
        }
        Update::NewMessage(msg) => {
            if let Some(mut m) = convert_message(&msg.message) {
                let tx2 = tx.clone();
                tokio::spawn(async move {
                    resolve_sender_name(&mut m, client_id).await;
                    let _ = tx2.send(AppEvent::NewMessage(m));
                });
            }
        }
        Update::MessageContent(upd) => {
            let new_text = extract_text_content(&upd.new_content);
            let _ = tx.send(AppEvent::MessageEdited(upd.chat_id, upd.message_id, new_text));
        }
        Update::DeleteMessages(upd) => {
            if upd.from_cache {
                return;
            }
            let _ = tx.send(AppEvent::MessagesDeleted(upd.chat_id, upd.message_ids));
        }
        Update::ChatReadInbox(upd) => {
            let _ = tx.send(AppEvent::ChatUnreadCount(upd.chat_id, upd.unread_count));
        }
        _ => {}
    }
}

fn handle_auth_state(
    state: tdlib_rs::enums::AuthorizationState,
    client_id: i32,
    config: &Config,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    use tdlib_rs::enums::AuthorizationState;

    match state {
        AuthorizationState::WaitTdlibParameters => {
            let data_dir = Config::data_dir().to_string_lossy().to_string();
            let api_id = config.general.effective_api_id();
            let api_hash = config.general.effective_api_hash();

            tokio::spawn(async move {
                let _ = tdlib_rs::functions::set_tdlib_parameters(
                    false,                                 // use_test_dc
                    data_dir.clone(),                      // database_directory
                    format!("{data_dir}/files"),           // files_directory
                    String::new(),                         // database_encryption_key
                    true,                                  // use_file_database
                    true,                                  // use_chat_info_database
                    true,                                  // use_message_database
                    false,                                 // use_secret_chats
                    api_id,                                // api_id
                    api_hash,                              // api_hash
                    "en".to_string(),                      // system_language_code
                    "kfs-tg".to_string(),                  // device_model
                    String::new(),                         // system_version
                    env!("CARGO_PKG_VERSION").to_string(), // application_version
                    client_id,
                )
                .await;
            });
        }
        AuthorizationState::WaitPhoneNumber => {
            let _ = tx.send(AppEvent::AuthStatePhone);
        }
        AuthorizationState::WaitCode(_) => {
            let _ = tx.send(AppEvent::AuthStateCode);
        }
        AuthorizationState::WaitPassword(_) => {
            let _ = tx.send(AppEvent::AuthStatePassword);
        }
        AuthorizationState::Ready => {
            let _ = tx.send(AppEvent::AuthStateReady);
            let tx2 = tx.clone();
            tokio::spawn(async move {
                load_chats(client_id, &tx2).await;
            });
        }
        _ => {}
    }
}

async fn load_chats(client_id: i32, tx: &mpsc::UnboundedSender<AppEvent>) {
    // Load first batch of chats
    let _ = tdlib_rs::functions::load_chats(None, 30, client_id).await;

    match tdlib_rs::functions::get_chats(None, 30, client_id).await {
        Ok(tdlib_rs::enums::Chats::Chats(chats_obj)) => {
            let mut chats = Vec::new();
            for chat_id in chats_obj.chat_ids {
                if let Ok(tdlib_rs::enums::Chat::Chat(chat)) =
                    tdlib_rs::functions::get_chat(chat_id, client_id).await
                {
                    let kind = match &chat.r#type {
                        tdlib_rs::enums::ChatType::Private(p) => {
                            ChatKind::Private { user_id: p.user_id }
                        }
                        tdlib_rs::enums::ChatType::Secret(s) => {
                            ChatKind::Private { user_id: s.user_id }
                        }
                        tdlib_rs::enums::ChatType::BasicGroup(g) => ChatKind::BasicGroup {
                            group_id: g.basic_group_id,
                        },
                        tdlib_rs::enums::ChatType::Supergroup(sg) => {
                            if sg.is_channel {
                                ChatKind::Channel
                            } else {
                                ChatKind::Supergroup {
                                    group_id: sg.supergroup_id,
                                }
                            }
                        }
                    };
                    chats.push(Chat {
                        id: chat.id,
                        title: chat.title,
                        unread_count: chat.unread_count,
                        last_message: chat
                            .last_message
                            .as_ref()
                            .map(|m| extract_text_content(&m.content)),
                        kind,
                    });
                }
            }
            let _ = tx.send(AppEvent::ChatsLoaded(chats));
        }
        _ => {
            let _ = tx.send(AppEvent::Error("Failed to load chats".to_string()));
        }
    }
}

pub async fn refresh_chats(client_id: i32, tx: &mpsc::UnboundedSender<AppEvent>) {
    load_chats(client_id, tx).await;
}

pub async fn load_chat_messages(
    chat_id: i64,
    client_id: i32,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match tdlib_rs::functions::get_chat_history(chat_id, 0, 0, 50, false, client_id).await {
        Ok(tdlib_rs::enums::Messages::Messages(msgs)) => {
            let mut messages: Vec<Message> = msgs
                .messages
                .into_iter()
                .flatten()
                .filter_map(|m| convert_message(&m))
                .collect();
            for msg in &mut messages {
                resolve_sender_name(msg, client_id).await;
            }
            // Mark messages as read
            let msg_ids: Vec<i64> = messages.iter().map(|m| m.id).collect();
            if !msg_ids.is_empty() {
                let _ = tdlib_rs::functions::view_messages(
                    chat_id, msg_ids, None, true, client_id,
                )
                .await;
            }
            let _ = tx.send(AppEvent::MessagesLoaded(messages));
        }
        _ => {
            let _ = tx.send(AppEvent::Error("Failed to load messages".to_string()));
        }
    }
}

pub async fn load_older_messages(
    chat_id: i64,
    from_message_id: i64,
    client_id: i32,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match tdlib_rs::functions::get_chat_history(chat_id, from_message_id, 0, 30, false, client_id)
        .await
    {
        Ok(tdlib_rs::enums::Messages::Messages(msgs)) => {
            let mut messages: Vec<Message> = msgs
                .messages
                .into_iter()
                .flatten()
                .filter_map(|m| convert_message(&m))
                .filter(|m| m.id != from_message_id)
                .collect();
            for msg in &mut messages {
                resolve_sender_name(msg, client_id).await;
            }
            let _ = tx.send(AppEvent::OlderMessagesLoaded(messages));
        }
        _ => {
            let _ = tx.send(AppEvent::OlderMessagesLoaded(Vec::new()));
        }
    }
}

pub async fn send_text_message(
    chat_id: i64,
    text: &str,
    reply_to_id: Option<i64>,
    client_id: i32,
) -> anyhow::Result<()> {
    let content =
        tdlib_rs::enums::InputMessageContent::InputMessageText(tdlib_rs::types::InputMessageText {
            text: tdlib_rs::types::FormattedText {
                text: text.to_string(),
                entities: Vec::new(),
            },
            link_preview_options: None,
            clear_draft: true,
        });

    let reply_to = reply_to_id.map(|msg_id| {
        tdlib_rs::enums::InputMessageReplyTo::Message(
            tdlib_rs::types::InputMessageReplyToMessage {
                message_id: msg_id,
                quote: None,
                checklist_task_id: 0,
            },
        )
    });

    tdlib_rs::functions::send_message(chat_id, None, reply_to, None, content, client_id)
        .await
        .map_err(|e| anyhow::anyhow!("send_message: {} (code {})", e.message, e.code))?;
    Ok(())
}

pub async fn edit_message_text(
    chat_id: i64,
    message_id: i64,
    text: &str,
    client_id: i32,
) -> anyhow::Result<()> {
    let content =
        tdlib_rs::enums::InputMessageContent::InputMessageText(tdlib_rs::types::InputMessageText {
            text: tdlib_rs::types::FormattedText {
                text: text.to_string(),
                entities: Vec::new(),
            },
            link_preview_options: None,
            clear_draft: true,
        });

    tdlib_rs::functions::edit_message_text(chat_id, message_id, content, client_id)
        .await
        .map_err(|e| anyhow::anyhow!("edit_message_text: {} (code {})", e.message, e.code))?;
    Ok(())
}

pub async fn submit_phone(phone: &str, client_id: i32) -> anyhow::Result<()> {
    tdlib_rs::functions::set_authentication_phone_number(phone.to_string(), None, client_id)
        .await
        .map_err(|e| anyhow::anyhow!("set_phone: {} (code {})", e.message, e.code))?;
    Ok(())
}

pub async fn submit_code(code: &str, client_id: i32) -> anyhow::Result<()> {
    tdlib_rs::functions::check_authentication_code(code.to_string(), client_id)
        .await
        .map_err(|e| anyhow::anyhow!("check_code: {} (code {})", e.message, e.code))?;
    Ok(())
}

pub async fn submit_password(password: &str, client_id: i32) -> anyhow::Result<()> {
    tdlib_rs::functions::check_authentication_password(password.to_string(), client_id)
        .await
        .map_err(|e| anyhow::anyhow!("check_password: {} (code {})", e.message, e.code))?;
    Ok(())
}

pub async fn get_bot_commands(kind: ChatKind, client_id: i32) -> Vec<(String, String)> {
    match kind {
        ChatKind::Private { user_id } => {
            match tdlib_rs::functions::get_user_full_info(user_id, client_id).await {
                Ok(tdlib_rs::enums::UserFullInfo::UserFullInfo(info)) => info
                    .bot_info
                    .map(|bi| {
                        bi.commands
                            .into_iter()
                            .map(|c| (c.command, c.description))
                            .collect()
                    })
                    .unwrap_or_default(),
                _ => Vec::new(),
            }
        }
        ChatKind::BasicGroup { group_id } => {
            match tdlib_rs::functions::get_basic_group_full_info(group_id, client_id).await {
                Ok(tdlib_rs::enums::BasicGroupFullInfo::BasicGroupFullInfo(info)) => info
                    .bot_commands
                    .into_iter()
                    .flat_map(|bc| bc.commands.into_iter().map(|c| (c.command, c.description)))
                    .collect(),
                _ => Vec::new(),
            }
        }
        ChatKind::Supergroup { group_id } => {
            match tdlib_rs::functions::get_supergroup_full_info(group_id, client_id).await {
                Ok(tdlib_rs::enums::SupergroupFullInfo::SupergroupFullInfo(info)) => info
                    .bot_commands
                    .into_iter()
                    .flat_map(|bc| bc.commands.into_iter().map(|c| (c.command, c.description)))
                    .collect(),
                _ => Vec::new(),
            }
        }
        ChatKind::Channel => Vec::new(),
    }
}

async fn resolve_sender_name(msg: &mut Message, client_id: i32) {
    if !msg.sender_name.starts_with("user:") {
        return;
    }
    let user_id: i64 = msg.sender_name["user:".len()..].parse().unwrap_or(0);
    if user_id == 0 {
        return;
    }
    if let Ok(tdlib_rs::enums::User::User(user)) =
        tdlib_rs::functions::get_user(user_id, client_id).await
    {
        msg.sender_name = if user.last_name.is_empty() {
            user.first_name
        } else {
            format!("{} {}", user.first_name, user.last_name)
        };
    }
}

fn convert_message(msg: &tdlib_rs::types::Message) -> Option<Message> {
    Some(Message {
        id: msg.id,
        chat_id: msg.chat_id,
        sender_name: extract_sender_name(&msg.sender_id),
        text: extract_text_content(&msg.content),
        timestamp: msg.date as i64,
        is_outgoing: msg.is_outgoing,
    })
}

fn extract_sender_name(sender: &tdlib_rs::enums::MessageSender) -> String {
    match sender {
        tdlib_rs::enums::MessageSender::User(u) => format!("user:{}", u.user_id),
        tdlib_rs::enums::MessageSender::Chat(c) => format!("chat:{}", c.chat_id),
    }
}

fn extract_text_content(content: &tdlib_rs::enums::MessageContent) -> String {
    match content {
        tdlib_rs::enums::MessageContent::MessageText(t) => t.text.text.clone(),
        tdlib_rs::enums::MessageContent::MessagePhoto(p) => {
            format!("[Photo] {}", p.caption.text)
        }
        tdlib_rs::enums::MessageContent::MessageVideo(v) => {
            format!("[Video] {}", v.caption.text)
        }
        tdlib_rs::enums::MessageContent::MessageDocument(d) => {
            format!("[File] {}", d.caption.text)
        }
        tdlib_rs::enums::MessageContent::MessageSticker(s) => {
            format!("[Sticker] {}", s.sticker.emoji)
        }
        tdlib_rs::enums::MessageContent::MessageVoiceNote(_) => "[Voice]".to_string(),
        tdlib_rs::enums::MessageContent::MessageVideoNote(_) => "[Video Note]".to_string(),
        tdlib_rs::enums::MessageContent::MessageAnimation(a) => {
            format!("[GIF] {}", a.caption.text)
        }
        _ => "[unsupported message]".to_string(),
    }
}
