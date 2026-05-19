mod chat_list;
mod login;
mod messages;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Panel, Screen};
use crate::keys::Mode;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    match app.screen {
        Screen::Login => login::draw(frame, app, chunks[0]),
        Screen::Main => draw_main(frame, app, chunks[0]),
    }

    draw_input(frame, app, chunks[1]);
    draw_status(frame, app, chunks[2]);
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.config.ui.chat_list_width),
            Constraint::Percentage(100 - app.config.ui.chat_list_width),
        ])
        .split(area);

    chat_list::draw(frame, app, h_chunks[0]);
    messages::draw(frame, app, h_chunks[1]);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
    };

    let border_style = if app.mode == Mode::Insert {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" [{mode_str}] "));

    let paragraph = Paragraph::new(app.input.as_str()).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let panel_str = match app.panel {
        Panel::ChatList => "chats",
        Panel::Messages => "messages",
    };
    let text = format!(
        " {} | {panel_str} | q:quit i:insert h/l:panel j/k:nav",
        app.status
    );
    let bar = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(bar, area);
}
