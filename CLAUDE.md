# CLAUDE.md

## Overview

Minimalist TUI Telegram client in Rust. Uses TDLib via rust-tdlib for Telegram API, ratatui + crossterm for terminal UI. Vim-like keybindings.

## Build & Test

```bash
cargo build                    # Dev build (first build compiles TDLib deps)
cargo build --release          # Release build
cargo test                     # Tests
cargo clippy -- -D warnings    # Lint
cargo fmt --check              # Format check
cargo run -- --config path     # Run with custom config
```

### Prerequisites
- Rust 1.75+
- TDLib shared library (libtdjson) installed on system
- Telegram API credentials (api_id + api_hash from https://my.telegram.org)

## Architecture

```
src/
  main.rs       — CLI (clap), terminal setup, tokio event loop
  app.rs        — App state machine, key handling, event dispatch
  config.rs     — TOML config loading (~/.config/kfs-tg/config.toml)
  keys.rs       — Vim-mode keybinding definitions (Normal/Insert)
  tg/
    mod.rs      — TDLib worker init, auth flow orchestration
    types.rs    — Domain types (Chat, Message, User)
  ui/
    mod.rs      — Layout dispatch, input bar, status bar
    chat_list.rs — Left panel: chat list with unread counts
    messages.rs  — Right panel: message history
    login.rs     — Auth flow screens (phone, code, 2FA)
```

## Key Design Decisions

- **Vim modes**: Normal (navigation) + Insert (typing). `i` enters insert, `Esc` exits.
- **Async**: tokio runtime, TDLib runs in spawned task, communicates via mpsc channel
- **TDLib**: rust-tdlib crate with `client` + `tokio` features
- **Config**: TOML at `~/.config/kfs-tg/config.toml`, XDG dirs via `dirs` crate

## CI/CD

- `.github/workflows/ci.yml` — Push/PR: fmt + clippy + test
- `.github/workflows/release.yml` — Tag `v*`: builds 5 targets (linux amd64/arm64, macOS amd64/arm64, windows amd64), creates GitHub Release, updates homebrew tap

## Current State (v0.1.0-dev)

### Done
- Project structure and all module stubs
- Config loading with TOML + defaults
- Vim keybindings (Normal/Insert mode)
- TUI layout (chat list, messages, input, status bar)
- Login screen UI
- CI + release pipeline (including Windows + Homebrew)

### TODO
- TDLib worker integration (actual auth + API calls)
- Real chat loading and message display
- Send messages
- Media handling
