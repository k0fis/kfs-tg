use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::{App, Panel};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.panel == Panel::ChatList {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let filtered = app.filtered_chat_indices();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&i| {
            let chat = &app.chats[i];
            let unread = if chat.unread_count > 0 {
                format!(" ({})", chat.unread_count)
            } else {
                String::new()
            };
            ListItem::new(format!("{}{unread}", chat.title))
        })
        .collect();

    let title = if app.search_active {
        format!(" Chats [/{}] ", app.search_query)
    } else {
        " Chats ".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    let mut state = ListState::default();
    if let Some(pos) = filtered.iter().position(|&i| i == app.chat_cursor) {
        state.select(Some(pos));
    }
    frame.render_stateful_widget(list, area, &mut state);
}
