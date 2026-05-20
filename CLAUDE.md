# CLAUDE.md

## Overview

Minimalist TUI Telegram client in Rust. Uses TDLib via `tdlib-rs` (static linking) for Telegram API, ratatui + crossterm for terminal UI. Vim-like keybindings with Normal/Insert modes.

## Build & Test

```bash
cargo build                    # Dev build (first build downloads+compiles TDLib ~10min)
cargo build --release          # Release build (strip + LTO)
cargo clippy -- -D warnings    # Lint
cargo fmt --check              # Format check
cargo run                      # Run (needs API credentials)
```

### Environment Variables (build time)
```bash
KFS_TG_API_ID=12345            # Telegram API ID (baked into binary)
KFS_TG_API_HASH=abc123         # Telegram API hash (baked into binary)
```
If not set at build time, falls back to config file values at runtime.

### Prerequisites
- Rust 1.75+
- C++17 compiler, CMake, OpenSSL, zlib (for TDLib static build)
- Telegram API credentials from https://my.telegram.org

## Architecture

```
src/
  main.rs       - CLI (clap), terminal setup, tokio event loop
  app.rs        - App state machine, key handling, event dispatch
  config.rs     - TOML config loading (~/.config/kfs-tg/config.toml)
  keys.rs       - Vim-mode keybinding definitions (Normal/Insert)
  tg/
    mod.rs      - TDLib receiver loop (spawn_blocking), auth flow, API calls
    types.rs    - Domain types (Chat, ChatKind, Message)
  ui/
    mod.rs      - Layout dispatch, input bar, status bar, popups (help, commands, forward)
    chat_list.rs - Left panel: chat list with unread counts + search filter
    messages.rs  - Right panel: message history with timestamps
    login.rs     - Auth flow screens (phone, code, 2FA)
```

## Key Design Decisions

- **Vim modes**: Normal (navigation) + Insert (typing). `i` enters insert, `Esc` exits.
- **Async**: tokio runtime. TDLib receiver in `spawn_blocking`, mpsc channel for events.
- **Static TDLib**: `tdlib-rs` v1.4 with `["static", "download-tdlib"]` — no system lib needed.
- **Compile-time credentials**: `env!("KFS_TG_API_ID")` baked into binary, fallback to config.
- **ChatKind enum**: Private/BasicGroup/Supergroup/Channel — used for bot commands routing.
- **Live updates**: Handles TDLib `UpdateNewMessage`, `UpdateMessageContent`, `UpdateDeleteMessages`, `UpdateChatReadInbox`.
- **Notifications**: Desktop notifications via `notify-rust` for messages in non-active chats.

## Keybindings

### Normal mode
| Key | Action |
|-----|--------|
| j/k | Move down/up |
| h/l | Switch panel (chats/messages) |
| Ctrl+d/u | Page down/up (10 items) |
| g/G | Top / Bottom |
| Enter | Open chat |
| i | Insert mode |
| / | Bot commands popup |
| Ctrl+f | Search/filter chats |
| r | Reply to message |
| e | Edit own message |
| o | Open media in system viewer |
| f | Forward message |
| d | Delete own message |
| Ctrl+r | Refresh chat list |
| ? | Help popup |
| q | Quit |

### Insert mode
| Key | Action |
|-----|--------|
| Enter | Send message (or save edit) |
| Shift+Enter | New line |
| Esc / Ctrl+c | Back to Normal |
| Left/Right | Move cursor |

## CI/CD

- `.github/workflows/ci.yml` — Push/PR: fmt + clippy + build
  - Uses dummy credentials: `KFS_TG_API_ID: "0"`, `KFS_TG_API_HASH: "placeholder"`
  - Linux deps: `libssl-dev zlib1g-dev libc++-dev libc++abi-dev`
- `.github/workflows/release.yml` — Tag `v*`: builds 4 targets, creates GitHub Release, updates Homebrew tap
  - Targets: x86_64-unknown-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc
  - Homebrew tap: `k0fis/homebrew-tap` via `repository-dispatch`

## Config (~/.config/kfs-tg/config.toml)

```toml
[general]
api_id = 12345
api_hash = "abc..."

[ui]
chat_list_width = 25       # percent
show_timestamps = true
date_format = "%H:%M"
notifications = true       # desktop notifications for new messages
```

## Current State (v0.2.0)

### Done
- Full TDLib integration (auth, chats, messages, send/edit/delete/forward)
- Vim-like TUI with Normal/Insert modes
- Chat list with unread counts, search/filter (Ctrl+f)
- Message display with timestamps, sender names resolved
- Send, reply, edit, delete, forward messages
- Bot commands popup (/ key) — fetches from UserFullInfo/GroupFullInfo
- Open media in system viewer (download + xdg-open/open)
- Message pagination (auto-load older on scroll to top)
- Multiline input (Shift+Enter), cursor movement (arrow keys)
- Live updates (new messages, edits, deletes, unread counts)
- Desktop notifications (notify-rust)
- Help popup (?)
- CI + release pipeline (4 platforms + Homebrew tap)
- Version display in status bar

### Future Ideas
- Inline image preview (sixel/kitty protocol)
- Chat folders/pinned chats
- Search within messages
- Sticker preview
- Voice message recording
