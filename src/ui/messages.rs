use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, Panel};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
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

    let title = if app.msg_search_active {
        format!(" {chat_title} [/{query}] ", query = app.msg_search_query)
    } else if !app.typing_status.is_empty() {
        format!(" {chat_title} — {} ", app.typing_status)
    } else {
        format!(" {chat_title} ")
    };

    let matches = app.msg_search_matches();
    let inner_width = area.width.saturating_sub(2) as usize;

    let mut items: Vec<ListItem> = Vec::new();
    let mut msg_to_display: Vec<usize> = Vec::new();
    let mut prev_date: Option<String> = None;
    let mut unread_inserted = false;

    for (i, msg) in app.messages.iter().enumerate() {
        let date = format_date(msg.timestamp);

        if prev_date.as_ref() != Some(&date) {
            items.push(
                ListItem::new(format!("--- {date} ---"))
                    .style(Style::default().fg(Color::DarkGray)),
            );
            prev_date = Some(date);
        }

        if !unread_inserted
            && let Some(last_read_id) = app.unread_from_id
            && msg.id > last_read_id
            && !msg.is_outgoing
        {
            items.push(ListItem::new("── unread ──").style(Style::default().fg(Color::Red)));
            unread_inserted = true;
        }

        msg_to_display.push(items.len());

        let prefix = if msg.is_outgoing {
            "You"
        } else {
            &msg.sender_name
        };
        let text = format!("[{}] {prefix}: {}", format_ts(msg.timestamp), msg.text);
        let style = if matches.contains(&i) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let lines = wrap_text(&text, inner_width);
        items.push(ListItem::new(Text::from(lines)).style(style));
    }

    let display_idx = if !msg_to_display.is_empty() {
        Some(msg_to_display[app.msg_cursor.min(msg_to_display.len().saturating_sub(1))])
    } else {
        None
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    app.msg_list_state.select(display_idx);
    frame.render_stateful_widget(list, area, &mut app.msg_list_state);
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

fn format_date(ts: i64) -> String {
    #[cfg(unix)]
    {
        let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
        unsafe { libc::localtime_r(&ts as *const i64, &mut tm) };
        format!(
            "{:04}-{:02}-{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday
        )
    }
    #[cfg(not(unix))]
    {
        let days = ts / 86400;
        format!("day-{days}")
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::from(text.to_owned())];
    }
    let mut lines = Vec::new();
    for line in text.split('\n') {
        if line.chars().count() <= width {
            lines.push(Line::from(line.to_owned()));
        } else {
            let chars: Vec<char> = line.chars().collect();
            for chunk in chars.chunks(width) {
                lines.push(Line::from(chunk.iter().collect::<String>()));
            }
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(String::new()));
    }
    lines
}
