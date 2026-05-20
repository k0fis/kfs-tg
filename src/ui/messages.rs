use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::{App, Panel};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.panel == Panel::Messages {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let chat_title = app
        .chats
        .get(app.chat_cursor)
        .map(|c| c.title.as_str())
        .unwrap_or("No chat selected");

    let items: Vec<ListItem> = app
        .messages
        .iter()
        .map(|msg| {
            let prefix = if msg.is_outgoing {
                "You"
            } else {
                &msg.sender_name
            };
            ListItem::new(format!(
                "[{}] {prefix}: {}",
                format_ts(msg.timestamp),
                msg.text
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {chat_title} "))
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    let mut state = ListState::default();
    if !app.messages.is_empty() {
        state.select(Some(app.msg_cursor));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn format_ts(ts: i64) -> String {
    #[cfg(unix)]
    {
        let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
        unsafe { libc::localtime_r(&ts as *const i64, &mut tm) };
        format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
    }
    #[cfg(not(unix))]
    {
        let secs = ts % 86400;
        format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
    }
}
