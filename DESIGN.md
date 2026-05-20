# kfs-tg

Minimalistický TUI Telegram klient v Rustu. Vim-like keyboard UX.

## Tech Stack

| Vrstva | Crate | Poznámka |
|--------|-------|----------|
| TUI framework | ratatui 0.29 + crossterm 0.28 | |
| Telegram API | tdlib-rs 1.4 | Static TDLib, auto-download |
| Async runtime | tokio (full) | |
| Config | toml + serde + dirs | XDG (~/.config/kfs-tg/) |
| Logging | tracing + tracing-appender | File output |
| Notifications | notify-rust | macOS/Linux/Windows |
| CLI | clap 4 | |

## UI Layout

```
┌─ kfs-tg ──────────────────────────────────────────┐
│ Chats          │ Chat: Alice                       │
│────────────────│───────────────────────────────────│
│ > Alice    (2) │ [14:30] Alice: Ahoj!             │
│   Bob          │ [14:31] You: Čau, jak se máš?    │
│   Work Group   │ [14:32] Alice: Dobře, díky       │
│   News Channel │                                   │
│                │                                   │
│────────────────────────────────────────────────────│
│ [INSERT] reply: Ahoj!                              │
│ > odpověď...                                       │
├────────────────────────────────────────────────────│
│ Connected | messages | q:quit i:insert ?:help | v0.2.0│
└────────────────────────────────────────────────────┘
```

## Implementované Features (v0.2.0)

- TDLib autentizace (telefon → kód → 2FA → session)
- Chat list s unread count, live aktualizace
- Search/filter chatů (Ctrl+f)
- Zobrazení zpráv s timestampy a resolved sender names
- Odeslání zpráv (Enter v Insert mode)
- Multiline zprávy (Shift+Enter)
- Reply na zprávu (r)
- Edit vlastních zpráv (e)
- Delete vlastních zpráv (d + potvrzení y/n)
- Forward zpráv (f + picker cílového chatu)
- Bot commands (/ → popup s příkazy)
- Open media v externím prohlížeči (o → download + open)
- Message pagination (auto-load starších při scrollu nahoru)
- Page scroll (Ctrl+d/u)
- Refresh chatů (Ctrl+r)
- Desktop notifikace (notify-rust, konfigurovatelné)
- Mark as read při otevření chatu
- Live message updates (edits, deletes z ostatních)
- Help popup (?)
- CI/CD: 4 platformy + Homebrew tap

## Build & Install

```bash
# Build from source
KFS_TG_API_ID=12345 KFS_TG_API_HASH=abc cargo build --release

# Install via Homebrew (macOS/Linux)
brew tap k0fis/tap
brew install kfs-tg
```

## Config

```toml
# ~/.config/kfs-tg/config.toml
[general]
api_id = 12345
api_hash = "your_api_hash"

[ui]
chat_list_width = 25
notifications = true
```

API credentials: https://my.telegram.org/apps

## CI/CD

- **CI**: push/PR → fmt + clippy + build (dummy credentials)
- **Release**: tag `v*` → binaries pro linux-amd64, macos-amd64, macos-arm64, windows-amd64
- **Homebrew**: auto-update `k0fis/homebrew-tap` formula po release
