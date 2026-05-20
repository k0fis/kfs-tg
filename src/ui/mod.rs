mod chat_list;
mod login;
mod messages;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

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

    if app.help_visible {
        draw_help(frame, frame.area());
    }
    if app.cmd_visible {
        draw_commands(frame, app, frame.area());
    }
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

    let title = if let Some((_, ref preview)) = app.reply_to {
        format!(" [{mode_str}] reply: {preview} ")
    } else {
        format!(" [{mode_str}] ")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let paragraph = Paragraph::new(app.input.as_str()).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let panel_str = match app.panel {
        Panel::ChatList => "chats",
        Panel::Messages => "messages",
    };
    let text = format!(
        " {} | {panel_str} | q:quit i:insert h/l:panel j/k:nav ?:help | v{}",
        app.status,
        env!("CARGO_PKG_VERSION")
    );
    let bar = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(bar, area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let help_text = "\
 NORMAL mode
 ───────────────────────
 j/k       Move down/up
 h/l       Switch panel
 Enter     Open chat
 i         Insert mode
 g/G       Top / Bottom
 /         Bot commands
 r         Reply
 f         Forward
 d         Delete
 Ctrl+r    Refresh
 q         Quit

 INSERT mode
 ───────────────────────
 Enter     Send message
 Esc       Back to Normal
 Ctrl+c    Cancel

 Press any key to close";

    let w = 30_u16;
    let h = 20_u16;
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    let popup = Rect::new(x, y, w.min(area.width), h.min(area.height));

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ");
    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

fn draw_commands(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};

    let max_cmd_len = app
        .bot_commands
        .iter()
        .map(|(c, _)| c.len() + 1)
        .max()
        .unwrap_or(8);

    let lines: Vec<Line> = app
        .bot_commands
        .iter()
        .enumerate()
        .map(|(i, (cmd, desc))| {
            let prefix = if i == app.cmd_cursor { "> " } else { "  " };
            let style = if i == app.cmd_cursor {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            Line::from(vec![Span::styled(
                format!("{prefix}/{cmd:<max_cmd_len$} {desc}"),
                style,
            )])
        })
        .collect();

    let h = (app.bot_commands.len() as u16 + 2).min(area.height.saturating_sub(4));
    let w = 50_u16.min(area.width.saturating_sub(4));
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    let popup = Rect::new(x, y, w, h);

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Bot Commands (Enter:select Esc:close) ");
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}
