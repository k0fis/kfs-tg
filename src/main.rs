mod app;
mod config;
mod keys;
mod tg;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use tokio::sync::mpsc;

use app::{App, AppEvent};
use config::Config;

#[derive(Parser)]
#[command(name = "kfs-tg", version, about = "Minimalist TUI Telegram client")]
struct Cli {
    #[arg(short, long, help = "Path to config file")]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load(cli.config.as_deref())?;

    // Suppress TDLib stderr logging before creating client
    #[cfg(unix)]
    suppress_stderr();

    let client_id = tdlib_rs::create_client();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvent>();

    let tg_tx = event_tx.clone();
    let tg_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = tg::run(client_id, tg_config, tg_tx).await {
            tracing::error!("TDLib error: {e}");
        }
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config, client_id, event_tx);

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                break;
            }
            if app.handle_key(key) {
                break;
            }
        }

        while let Ok(ev) = event_rx.try_recv() {
            app.handle_event(ev);
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

#[cfg(unix)]
fn suppress_stderr() {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;
    if let Ok(devnull) = File::open("/dev/null") {
        unsafe {
            libc::dup2(devnull.as_raw_fd(), 2);
        }
    }
}
