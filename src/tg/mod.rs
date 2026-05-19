pub mod types;

use tokio::sync::mpsc;

use crate::app::AppEvent;
use crate::config::Config;

pub async fn run(_config: Config, tx: mpsc::UnboundedSender<AppEvent>) -> anyhow::Result<()> {
    // TODO: Initialize TDLib worker with rust-tdlib
    // For now, send a placeholder error so the app doesn't hang
    tx.send(AppEvent::AuthStatePhone)?;

    // Keep the task alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
