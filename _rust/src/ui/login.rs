use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, AuthState};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let prompt = match app.auth_state {
        AuthState::WaitPhone => "Enter your phone number (with country code):",
        AuthState::WaitCode => "Enter the verification code from Telegram:",
        AuthState::WaitPassword => "Enter your 2FA password:",
        AuthState::Ready => "Authenticated! Loading chats...",
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "kfs-tg",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(prompt),
        Line::from(""),
        Line::from(Span::styled(
            format!("> {}_", app.input),
            Style::default().fg(Color::Yellow),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Login ")
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
