mod chat_list;
mod login;
mod messages;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, Panel, Screen};
use crate::keys::Mode;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let inner_width = frame.area().width.saturating_sub(2) as usize;
    let input_lines = if app.input.is_empty() || inner_width == 0 {
        1u16
    } else {
        app.input
            .split('\n')
            .map(|line| {
                let chars = line.chars().count();
                1 + (chars / inner_width) as u16
            })
            .sum()
    };
    let input_height = (input_lines + 2).min(7);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(input_height),
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
    if app.forward_msg.is_some() {
        draw_forward_picker(frame, app, frame.area());
    }
}

fn draw_main(frame: &mut Frame, app: &mut App, area: Rect) {
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
    use ratatui::text::Line as TLine;

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
    } else if app.edit_msg.is_some() {
        format!(" [{mode_str}] editing ")
    } else {
        format!(" [{mode_str}] ")
    };

    let inner_w = area.width.saturating_sub(2) as usize;
    let visible_lines = area.height.saturating_sub(2);

    let wrapped_lines = wrap_input(&app.input, inner_w);

    let (cursor_visual_line, cursor_visual_col) = if inner_w > 0 {
        let text_before_cursor = &app.input[..app.input_cursor];
        let mut vline = 0u16;
        let logical_lines: Vec<&str> = text_before_cursor.split('\n').collect();
        for (i, line) in logical_lines.iter().enumerate() {
            let chars = line.chars().count();
            if i < logical_lines.len() - 1 {
                vline += if chars == 0 {
                    1
                } else {
                    ((chars as u16).saturating_sub(1)) / inner_w as u16 + 1
                };
            } else {
                vline += chars as u16 / inner_w as u16;
            }
        }
        let last_chars = logical_lines.last().map(|l| l.chars().count()).unwrap_or(0);
        let vcol = (last_chars % inner_w) as u16;
        (vline, vcol)
    } else {
        (0, 0)
    };

    let scroll_y = cursor_visual_line.saturating_sub(visible_lines.saturating_sub(1));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let text = ratatui::text::Text::from(
        wrapped_lines
            .into_iter()
            .map(TLine::from)
            .collect::<Vec<_>>(),
    );
    let paragraph = Paragraph::new(text).block(block).scroll((scroll_y, 0));
    frame.render_widget(paragraph, area);

    if app.mode == Mode::Insert {
        let draw_line = cursor_visual_line - scroll_y;
        frame.set_cursor_position((area.x + 1 + cursor_visual_col, area.y + 1 + draw_line));
    }
}

fn wrap_input(input: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![input.to_owned()];
    }
    let mut lines = Vec::new();
    for line in input.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            lines.push(String::new());
        } else {
            for chunk in chars.chunks(width) {
                lines.push(chunk.iter().collect());
            }
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    if app.open_chat_active {
        let text = format!(" Open chat: @{}_", app.open_chat_query);
        let bar = Paragraph::new(text).style(Style::default().fg(Color::Cyan));
        frame.render_widget(bar, area);
        return;
    }

    let panel_str = match app.panel {
        Panel::ChatList => "chats",
        Panel::Messages => "messages",
    };
    let folder_str = if app.folders.is_empty() {
        String::new()
    } else {
        let active = app
            .active_folder
            .and_then(|id| app.folders.iter().position(|f| f.0 == id))
            .map(|i| format!("[{}]", app.folders[i].1))
            .unwrap_or_else(|| "[All]".to_string());
        format!(" {active}")
    };
    let text = format!(
        " {} |{folder_str} {panel_str} | ?:help | v{}",
        app.status,
        env!("KFS_TG_VERSION")
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
 Ctrl+d/u  Page dn/up
 g/G       Top / Bottom
 Enter     Open chat
 i         Insert mode
 /         Bot commands
 Ctrl+f    Search chats
 Ctrl+s    Search messages
 Ctrl+o    Open @username
 r         Reply
 e         Edit message
 o         Open media
 f         Forward
 d         Delete
 Ctrl+r    Refresh
 q         Quit

 INSERT mode
 ───────────────────────
 Enter     Send message
 Ctrl+n    New line
 Esc       Back to Normal
 Ctrl+c    Cancel

 Press any key to close";

    let w = 30_u16;
    let h = 21_u16;
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

    let filtered = app.filtered_bot_commands();

    let max_cmd_len = filtered
        .iter()
        .map(|(c, _)| c.len() + 1)
        .max()
        .unwrap_or(8);

    let lines: Vec<Line> = filtered
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

    let title = if app.cmd_filter.is_empty() {
        " Bot Commands (Enter:select Esc:close) ".to_string()
    } else {
        format!(" /{} ", app.cmd_filter)
    };

    let h = (filtered.len() as u16 + 2).min(area.height.saturating_sub(4));
    let w = 50_u16.min(area.width.saturating_sub(4));
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    let popup = Rect::new(x, y, w, h);

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(title);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

fn draw_forward_picker(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};

    let lines: Vec<Line> = app
        .chats
        .iter()
        .enumerate()
        .map(|(i, chat)| {
            let prefix = if i == app.forward_cursor { "> " } else { "  " };
            let style = if i == app.forward_cursor {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else {
                Style::default()
            };
            Line::from(vec![Span::styled(format!("{prefix}{}", chat.title), style)])
        })
        .collect();

    let h = (app.chats.len() as u16 + 2).min(area.height.saturating_sub(4));
    let w = 45_u16.min(area.width.saturating_sub(4));
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    let popup = Rect::new(x, y, w, h);

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Forward to (Enter:send Esc:cancel) ");
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}
