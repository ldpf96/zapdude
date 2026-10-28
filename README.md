# Zapdude

**Zapdude** is a multi account WhatsApp Web desktop client for Windows,
written in **Rust** (Tauri 2 + WebView2). All your WhatsApp accounts live in
a single window, with a local API so people and AI agents can operate them.

**Original project by [ldpf96](https://github.com/ldpf96)**, developed with
an honest amount of AI assistance: the **OpenCode** agent harness driving
the **GLM-5.3** model (by Z.ai), in the casual "vibe coding" style, with
every feature verified on real hardware before shipping. The full origin
story lives in [`NOTICE.md`](NOTICE.md).

The starting point was the design and behavior of
[ZapZap](https://github.com/rafatosta/zapzap) by Rafael Tosta: early
versions deliberately tried to feel like ZapZap. The project then grew into
something substantially different, including a local automation API,
instant account switching, full screen options and call pop-outs. No ZapZap
code text ships here (different language, different stack), and we credit
the original concept loudly because that is the honest thing to do.
License: **AGPL-3.0**.

![Windows 10/11](https://img.shields.io/badge/Windows-10%2F11-0078d7) ![Rust](https://img.shields.io/badge/Rust-stable-dea584) ![Tauri 2](https://img.shields.io/badge/Tauri-2-FFC131) ![License](https://img.shields.io/badge/License-AGPL--3.0-blue)

## Features

- **Multi account at its core** - a left rail with a logo, one avatar per
  account and a "+". Adding an account asks for nothing: it opens its QR
  right away. Each account gets an **isolated WebView2 profile**, so
  sessions never mix.
- **Instant account switching** - inactive accounts are parked outside the
  window at full size, so switching is only a position move: WhatsApp does
  not re-render (measured: full paint in under 350 ms).
- **Fast startup with many accounts** - only the account you open loads
  WhatsApp Web; the rest are created the first time you open them.
- **Full screen options** (the logo button): API settings with live status,
  AI agent documentation and About, all inside a single window with no
  auxiliary windows.
- **System tray** with "Show <account>" per account, hide everything and
  quit.
- **WhatsApp Web calls** - stable Chrome identity (UA + a JS client hints
  patch) and pre-approved permissions for microphone, camera, screen
  sharing, notifications and clipboard.
- **Promo cleanup** - the "Download WhatsApp for Windows" landing banner
  and the download card in the empty chat panel remove themselves.
- **No terminal, no friction** - `windows_subsystem`, a single window that
  hides to the tray, single instance (mutex), NSIS installer.

## Installation (Windows 10/11)

Download `Zapdude_1.1.0_x64-setup.exe` from the releases and run it. It
installs for the current user (no admin needed). Uninstalling also removes
the app data.

Or build it yourself:

```powershell
cd zapdude-rs
.\tools\tauri.exe build --bundles nsis   # installer lands in target\release\bundle\nsis
```

## Configuration

`%APPDATA%\app.zapzap.desktop\config.json` (created on first run):

```json
{
  "api": {
    "enabled": false,
    "port": 29977,
    "apiKey": "zpk_...auto-generated...",
    "exposeNetwork": false,
    "webhookUrl": "",
    "webhookSecret": ""
  },
  "prefs": {
    "theme": "dark",
    "autostart": false,
    "startMinimized": false
  },
  "accounts": [
    { "id": "work", "label": "Work", "enabled": true }
  ]
}
```

- **API is disabled by default** (privacy): enable it from the options
  screen (logo, then API settings) or with `"enabled": true`.
- The `apiKey` is generated randomly on first run.
- `exposeNetwork: true` listens on `0.0.0.0` (only do this if you know what
  you are doing).

## Local API (v1)

Base `http://127.0.0.1:<port>/api`, auth via `X-API-Key` (or
`Authorization: Bearer`), 60 requests per minute, body up to 1 MB.

| Method | Route | Description |
|---|---|---|
| GET  | `/api/healthz` | Availability probe (no key) |
| GET  | `/api/accounts` | Configured accounts and their state |
| POST | `/api/accounts` | Adds an account and opens its QR |
| DELETE | `/api/accounts/{id}` | Removes an account (and its profile) |
| POST | `/api/accounts/{id}/open` | Shows that account in the window |
| GET  | `/api/status?account=<id>` | App + page + `accounts[]` |
| POST | `/api/open-chat` | `{"account","phone","text?"}` |
| POST | `/api/message` | `{"account","phone","message"}` |
| GET  | `/api/chats?limit=20&account=<id>` | Visible chat list (best effort) |
| GET  | `/api/events?since=<id>` | Recent event history |
| SSE  | `/api/events/stream` | Live events |
| GET/POST | `/api/webhook` | HTTPS event receiver (HMAC signed) |

Examples:

```bash
# Is the API alive?
curl http://127.0.0.1:29977/api/healthz

# State (session_state: detecting | scan_qr | logged_in)
curl -H "X-API-Key: $KEY" "http://127.0.0.1:29977/api/status"

# Send a message
curl -X POST -H "X-API-Key: $KEY" -H "Content-Type: application/json" \
  -d '{"account":"work","phone":"15551234567","message":"Hello from Rust"}' \
  http://127.0.0.1:29977/api/message

# Live events (SSE)
curl -N -H "X-API-Key: $KEY" http://127.0.0.1:29977/api/events/stream
```

## How it works

A single Tauri window hosts several webviews: the **rail** (76 px), the
full screen **options** panel (created on demand), a welcome backdrop and
one WhatsApp Web webview per account (isolated WebView2 profiles under
`%APPDATA%\app.zapzap.desktop\webview\<id>`).

The whole shell UI (rail, options, welcome) is served by the loopback API
server itself (same origin), so UI `fetch` calls never fight CORS and no
Tauri IPC is used for the UI.

Automation (read chats, open conversations, send messages) uses a resident
bridge injected into the page with a URL hash beacon transport; every
failure degrades to an API error without breaking the app.

```
zapdude-rs/
+-- src/window.rs      # multi webview shell + tray + lifecycle
+-- src/api/           # axum server, protocol, page bridge, events
+-- scripts/           # client hints, resident bridge, promo cleanup
+-- ui/                # rail, options (drawer) and welcome
+-- windows/           # NSIS hooks (cleanup on uninstall)
+-- docs/              # architecture and session notes
```

## Development

```powershell
cargo run                               # debug
cargo test                              # unit tests
.\tools\tauri.exe build --bundles nsis  # release + installer
```

Requires the stable Rust toolchain and the WebView2 Runtime (bundled with
Windows 11).

## Known limitations

- WhatsApp does not publish an official API: automation uses the real page
  and is **best effort** against their UI changes.
- The `Sec-CH-UA` HTTP headers cannot be rewritten from Tauri; the Chrome
  identity is applied in JavaScript, which is what WhatsApp Web's feature
  gates read.
- Background accounts load on first use: their live notifications start
  once they are opened in the session.

## License

**AGPL-3.0** - see [`LICENSE`](LICENSE) and [`NOTICE.md`](NOTICE.md).
Copyright belongs to [ldpf96](https://github.com/ldpf96). The design and
behavior reference was ZapZap by Rafael Tosta, who is credited with the
original concept. Zapdude is not an official WhatsApp product, is not built
by or affiliated with Meta, and the project keeps its own voice: started on
ZapZap's ideas, ended somewhere of its own.

AI notice: developed with **OpenCode** + the **GLM-5.3** model (by Z.ai),
vibe coding style, author reviewed and shipped every patch.
