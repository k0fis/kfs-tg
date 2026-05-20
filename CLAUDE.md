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

### Version
Version is derived from git tag at build time via `build.rs` (`git describe --tags`). No need to update Cargo.toml version manually.

### Prerequisites
- Rust 1.75+
- C++17 compiler, CMake, OpenSSL, zlib (for TDLib static build)
- Telegram API credentials from https://my.telegram.org

## Architecture

```
src/
  main.rs       - CLI (clap), terminal setup, tokio event loop, bracketed paste
  app.rs        - App state machine, key handling, event dispatch
  config.rs     - TOML config loading (~/.config/kfs-tg/config.toml)
  keys.rs       - Vim-mode keybinding definitions (Normal/Insert)
  build.rs      - TDLib build + git version extraction
  tg/
    mod.rs      - TDLib receiver loop (spawn_blocking), auth flow, API calls
    types.rs    - Domain types (Chat, ChatKind, Message)
  ui/
    mod.rs      - Layout dispatch, input bar, status bar, popups (help, commands, forward)
    chat_list.rs - Left panel: chat list with unread counts + search filter
    messages.rs  - Right panel: message history with timestamps, date separators, unread marker
    login.rs     - Auth flow screens (phone, code, 2FA)
```

## Key Design Decisions

- **Vim modes**: Normal (navigation) + Insert (typing). `i` enters insert, `Esc` exits.
- **Async**: tokio runtime. TDLib receiver in `spawn_blocking`, mpsc channel for events.
- **Static TDLib**: `tdlib-rs` v1.4 with `["static", "download-tdlib"]` — no system lib needed.
- **Compile-time credentials**: `env!("KFS_TG_API_ID")` baked into binary, fallback to config.
- **Version from git**: `build.rs` reads `git describe --tags`, sets `KFS_TG_VERSION` env.
- **ChatKind enum**: Private/BasicGroup/Supergroup/Channel — used for bot commands routing.
- **Live updates**: Handles TDLib `UpdateNewMessage`, `UpdateMessageContent`, `UpdateDeleteMessages`, `UpdateChatReadInbox`, `UpdateUserStatus`, `UpdateChatAction`, `UpdateChatFolders`.
- **Notifications**: Desktop notifications via `notify-rust` for messages in non-active chats.
- **Bracketed paste**: Supports multiline paste from clipboard.
- **Message ordering**: Messages sorted by timestamp after TDLib load; date separators between days.
- **Persistent ListState**: Scroll position preserved between frames for smooth navigation.

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
| Ctrl+s | Search messages |
| r | Reply to message |
| e | Edit own message |
| o | Open media in system viewer |
| f | Forward message |
| d | Delete own message |
| Ctrl+r | Refresh chat list |
| 0-9 | Switch chat folder |
| ? | Help popup |
| q | Quit |

### Insert mode
| Key | Action |
|-----|--------|
| Enter | Send message (or save edit) |
| Ctrl+n | New line |
| Esc / Ctrl+c | Back to Normal |
| Left/Right | Move cursor |

## CI/CD

- `.github/workflows/ci.yml` — Push/PR: fmt + clippy + build
  - Uses dummy credentials: `KFS_TG_API_ID: "0"`, `KFS_TG_API_HASH: "placeholder"`
  - Linux deps: `libssl-dev zlib1g-dev libc++-dev libc++abi-dev`
- `.github/workflows/release.yml` — Tag `v*`: builds 4 targets + .deb, creates GitHub Release, updates Homebrew tap + APT repo
  - Targets: x86_64-linux, x86_64-macos, aarch64-macos, x86_64-windows
  - Homebrew tap: `k0fis/homebrew-tap` via `repository-dispatch`
  - APT repo: `k0fis/apt` via `repository-dispatch` (GitHub Pages)
  - Actions: checkout@v6, upload-artifact@v7, download-artifact@v8

## Distribution

### Homebrew (macOS/Linux)
```bash
brew tap k0fis/tap
brew install kfs-tg
```

### APT (Debian/Ubuntu)
```bash
curl -fsSL https://k0fis.github.io/apt/gpg.key | sudo gpg --dearmor -o /usr/share/keyrings/k0fis.gpg
echo "deb [signed-by=/usr/share/keyrings/k0fis.gpg] https://k0fis.github.io/apt stable main" | sudo tee /etc/apt/sources.list.d/k0fis.list
sudo apt update && sudo apt install kfs-tg
```

## Config (~/.config/kfs-tg/config.toml)

```toml
[general]
api_id = 12345
api_hash = "abc..."

[ui]
chat_list_width = 20       # percent
notifications = true       # desktop notifications for new messages
```

## Current State (v0.4.2)

### Done
- Full TDLib integration (auth, chats, messages, send/edit/delete/forward)
- Vim-like TUI with Normal/Insert modes
- Chat list with unread counts, search/filter (Ctrl+f)
- Message display with timestamps, date separators (yyyy-mm-dd), unread marker
- Messages sorted chronologically, persistent scroll state
- Long message wrapping (character-based)
- Send, reply, edit, delete, forward messages
- Bot commands popup (/ key) — fetches from UserFullInfo/GroupFullInfo
- Open media in system viewer (download + xdg-open/open)
- Message pagination (auto-load older on scroll to top)
- Message search (Ctrl+s) with highlighting
- Multiline input (Ctrl+N for new line), bracketed paste support
- Live updates (new messages, edits, deletes, unread counts, typing, user status)
- Chat folders (0-9 to switch)
- Desktop notifications (notify-rust)
- Help popup (?)
- Version derived from git tag at build time
- CI + release pipeline (4 platforms + Homebrew tap + APT repo)
- Process exits cleanly on quit

### Future Ideas
- Inline image preview (sixel/kitty protocol)
- Sticker preview
- Voice message recording
- TG bot for RSS reading (kfsRss integration)
