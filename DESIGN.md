# kfs-tg — Design Document

Minimalistický TUI Telegram klient v Rustu. Vim-like keyboard UX, inspirace z [tgt](https://github.com/FedericoBruzzone/tgt) ale vlastní implementace.

## Tech Stack

| Vrstva | Crate | Poznámka |
|--------|-------|----------|
| TUI framework | ratatui + crossterm | Stejný stack jako kfs-squid-editor |
| Telegram API | rust-tdlib (v0.4.3) | TDLib v1.8.0 binding, tokio async |
| Async runtime | tokio | rust-tdlib to vyžaduje |
| Config | toml + dirs | XDG config (~/.config/kfs-tg/) |
| Logging | tracing | Async-friendly, file output |

### TDLib build
- Feature `download-tdlib` stáhne a zkompiluje TDLib automaticky při `cargo build`
- Vyžaduje: CMake, C++17 compiler, OpenSSL, zlib, gperf
- Alternativa: pkg-config s předkompilovaným TDLib

## Architektura

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   TUI       │◄───►│   App State  │◄───►│  TDLib      │
│  (ratatui)  │     │  (messages,  │     │  Worker     │
│             │     │   chats)     │     │  (async)    │
└─────────────┘     └──────────────┘     └─────────────┘
     ▲                                         ▲
     │                                         │
  crossterm                              rust-tdlib
  events                                 updates
```

### Moduly

```
src/
  main.rs          — CLI args, terminal setup, tokio runtime
  app.rs           — App state machine, event dispatch
  tg/
    mod.rs         — TDLib worker init, auth flow
    client.rs      — High-level API (send msg, load chats, download media)
    updates.rs     — Update handler (new messages, status changes)
    types.rs       — Domain types (Chat, Message, User) zjednodušené z TDLib
  ui/
    mod.rs         — Layout dispatch
    chat_list.rs   — Levý panel: seznam chatů
    messages.rs    — Pravý panel: zprávy v chatu
    input.rs       — Spodní panel: compose message
    status.rs      — Status bar
    login.rs       — Auth/login flow UI
  config.rs        — TOML config loading
  keys.rs          — Keybinding definitions + vim-mode state
```

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
│                │                                   │
│────────────────────────────────────────────────────│
│ > Type message...                          [NORMAL]│
└────────────────────────────────────────────────────┘
```

### Panely
- **Chat list** (levý, ~25% šířky): seznam chatů, unread count, poslední zpráva
- **Messages** (pravý): zprávy v aktivním chatu, scroll, reply indikátor
- **Input** (dole): compose zpráva, vim modes (NORMAL/INSERT)
- **Status bar** (úplně dole): connection status, mode, keybind hints

## Vim-like Keybindings

### Normal mode (navigace)
| Key | Akce |
|-----|------|
| `j/k` | Pohyb v chat listu / zprávách |
| `h/l` | Přepínání panelů (chats ↔ messages) |
| `i` | Enter INSERT mode (compose) |
| `Enter` | Otevřít chat / expand |
| `gg/G` | Top/bottom |
| `/` | Hledat v chatech |
| `r` | Reply na vybranou zprávu |
| `f` | Forward |
| `d` | Delete zpráva (s potvrzením) |
| `q` | Quit |
| `Ctrl+r` | Refresh |

### Insert mode (psaní)
| Key | Akce |
|-----|------|
| `Esc` | Zpět do NORMAL |
| `Enter` | Odeslat zprávu |
| `Shift+Enter` | Nový řádek |
| `Ctrl+c` | Cancel (zpět do NORMAL) |

## Auth Flow

TDLib řídí autentizaci sám. Klient reaguje na stavy:
1. `AuthorizationStateWaitPhoneNumber` → input pro telefon
2. `AuthorizationStateWaitCode` → input pro SMS kód
3. `AuthorizationStateWaitPassword` → 2FA heslo
4. `AuthorizationStateReady` → hlavní UI

Session se ukládá v `~/.local/share/kfs-tg/tdlib/` (TDLib database).

## Config (~/.config/kfs-tg/config.toml)

```toml
[general]
api_id = 12345          # z https://my.telegram.org
api_hash = "abc..."

[ui]
chat_list_width = 25    # procenta
show_timestamps = true
date_format = "%H:%M"
theme = "dark"

[keys]
# Override keybindings (future)
```

## MVP Milestones

### v0.1.0 — Login + Chat list
- [ ] Projekt init (cargo, deps, CI)
- [ ] TDLib worker setup + auth flow
- [ ] Login UI (phone + code + 2FA)
- [ ] Chat list loading a zobrazení
- [ ] Základní navigace (j/k, Enter)

### v0.2.0 — Read messages
- [ ] Message history loading
- [ ] Scroll v messages panelu
- [ ] Unread count + mark as read
- [ ] Reply/forward indikátory v messages

### v0.3.0 — Send messages
- [ ] INSERT mode + compose
- [ ] Odeslání textové zprávy
- [ ] Reply na konkrétní zprávu
- [ ] Edit/delete vlastní zprávy

### v0.4.0 — Media & polish
- [ ] Stahování médií (fotky, soubory) → otevření v externím prohlížeči
- [ ] Inline image preview (sixel/kitty protocol, optional)
- [ ] Notifikace (desktop notification via notify-rust)
- [ ] Search v zprávách

## Build & Run

```bash
cargo build                          # Debug (stáhne + kompiluje TDLib ~5min poprvé)
cargo build --release                # Release
cargo run -- --config ~/.config/kfs-tg/config.toml
```

### Prerekvizity
- Rust 1.75+
- CMake 3.10+
- C++17 compiler (gcc 7+ / clang 5+)
- OpenSSL, zlib, gperf
- Telegram API credentials (api_id + api_hash z https://my.telegram.org)

## CI/CD

- GitHub Actions: fmt + clippy + test (bez TDLib integračních testů)
- Release: tag `v*` → linux amd64/arm64 + macOS amd64/arm64
- TDLib se cachuje v CI (CMake build ~5min)

## Rizika & Gotchas

1. **rust-tdlib v0.4.3 podporuje jen TDLib v1.8.0** — novější TDLib features nedostupné
2. **Build time** — první kompilace TDLib trvá 5-10 minut
3. **TDLib session management** — crash recovery, multiple devices
4. **Rate limits** — Telegram může limitovat při flood
5. **Media v terminálu** — omezené možnosti (sixel/kitty jen v některých terminálech)

## Reference

- [rust-tdlib docs](https://docs.rs/rust-tdlib/)
- [TDLib docs](https://core.telegram.org/tdlib)
- [tgt (inspirace)](https://github.com/FedericoBruzzone/tgt)
- [Telegram API credentials](https://my.telegram.org/apps)
