pub mod types;

use tokio::sync::mpsc;

use crate::app::AppEvent;
use crate::config::Config;
use crate::tg::types::{Chat, Message};

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

    // Send initial request to start receiving updates
    tdlib_rs::functions::set_log_verbosity_level(1, client_id)
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
            if let Some(m) = convert_message(&msg.message) {
                let _ = tx.send(AppEvent::NewMessage(m));
            }
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
            let api_id = config.general.api_id;
            let api_hash = config.general.api_hash.clone();

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
                    chats.push(Chat {
                        id: chat.id,
                        title: chat.title,
                        unread_count: chat.unread_count,
                        last_message: chat
                            .last_message
                            .as_ref()
                            .map(|m| extract_text_content(&m.content)),
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

pub async fn load_chat_messages(
    chat_id: i64,
    client_id: i32,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match tdlib_rs::functions::get_chat_history(chat_id, 0, 0, 50, false, client_id).await {
        Ok(tdlib_rs::enums::Messages::Messages(msgs)) => {
            let messages: Vec<Message> = msgs
                .messages
                .into_iter()
                .flatten()
                .filter_map(|m| convert_message(&m))
                .collect();
            let _ = tx.send(AppEvent::MessagesLoaded(messages));
        }
        _ => {
            let _ = tx.send(AppEvent::Error("Failed to load messages".to_string()));
        }
    }
}

pub async fn send_text_message(chat_id: i64, text: &str, client_id: i32) -> anyhow::Result<()> {
    let content =
        tdlib_rs::enums::InputMessageContent::InputMessageText(tdlib_rs::types::InputMessageText {
            text: tdlib_rs::types::FormattedText {
                text: text.to_string(),
                entities: Vec::new(),
            },
            link_preview_options: None,
            clear_draft: true,
        });

    tdlib_rs::functions::send_message(chat_id, None, None, None, content, client_id)
        .await
        .map_err(|e| anyhow::anyhow!("send_message: {} (code {})", e.message, e.code))?;
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
