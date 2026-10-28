# Architecture - Zapdude

Living document. Describes the components and design decisions of the
project (original code by ldpf96; the starting design reference was the
ZapZap concept by Rafael Tosta, see NOTICE.md).

## Overview

```
+----------------------------------------------------------------------+
|  zapdude.exe (Tauri 2 + WebView2)                                    |
|                                                                      |
|  +----------------------------------------------------------------+  |
|  | main window (single window shell)                              |  |
|  |                                                                |  |
|  |  [ rail 76px ]   [ welcome / account webviews / drawer ]       |  |
|  |                                                                |  |
|  |  rail.html      : logo (toggles drawer), avatars, "+"          |  |
|  |  account-<id>   : one WhatsApp Web webview per account         |  |
|  |  drawer         : full screen options panel, created on demand |  |
|  |  welcome        : branded backdrop / empty-state destination   |  |
|  +----------------------------------------------------------------+  |
|                                                                      |
|  [ tray ] Show <account>, Hide all windows, Quit Zapdude             |
|                                                                      |
|  [ loopback API server ] REST + SSE + signed webhook                 |
+----------------------------------------------------------------------+
```

Call pop-outs open as separate windows. They share the account's WebView2
environment, so the session and cookies are the same one the page uses.

## Components

| Piece | Role |
|---|---|
| `src/main.rs` | startup order: single instance, server, shell, tray, first account |
| `src/window.rs` | the webview shell: rail + parked accounts + drawer + tray + popouts |
| `src/config.rs` | settings on disk: `api` + `prefs` + `accounts`, sanitized, BOM tolerant |
| `src/api/mod.rs` | module glue for the automation API |
| `src/api/protocol.rs` | auth (`X-API-Key`/Bearer), rate limit (60/60s), body caps, HMAC signing, phone normalization |
| `src/api/server.rs` | axum 0.8 loopback server, shell UI router, config and preferences endpoints |
| `src/api/page_ops.rs` | per webview bridge (`window.__zapdudeApiV1`, page info, composer ops, chat list) and its beacon poller |
| `src/api/events.rs` | event ring, SSE broadcast, webhook delivery |
| `scripts/client_hints_fix.js` | JS side Chrome identity (Sec-CH-UA is HTTP layer and not rewritable from the host) |
| `scripts/api_page_bridge.js` | resident per document: `window.__zapdudeApiV1`, Notification hook to events, `#__zapdude=` beacon fallback |
| `scripts/hide_download_banners.js` | removes the "download the app" promos with guardrails |
| `ui/rail.html` | left rail: logo toggles the drawer, avatars with state dots, right click delete, "+" |
| `ui/drawer.html` | full screen options: preferences, account rename, API settings, agent docs, About |
| `ui/welcome.html` | branded backdrop and the empty-state destination when no accounts remain |
| `windows/installer-hooks.nsh` | NSIS hook that wipes `%APPDATA%\app.zapzap.desktop` on uninstall |

## Startup order (critical)

1. Single instance: Windows mutex `Local\app.zapzap.desktop.single`; a
   second instance exits immediately.
2. The loopback server starts BEFORE any webview exists (the UI is served
   by the same server, and a UI that boots before the server returns
   broken pages).
3. `create_shell`: rail + welcome webviews.
4. Accounts: the first account opens right away (or "Account 1" is
   created on a virgin install with no config). Every other account
   creates its webview lazily on first open.
5. Tray icon and menu.

## Shells and visibility model

- One Tauri window (`main`, feature `unstable`) hosts several child
  webviews. The ACTIVE content webview sits next to the rail at
  `(RAIL_WIDTH, 0)` at full size; every other content webview is parked
  outside the client area (x = -24000 logical) AT FULL SIZE.
- Why full-size parking: a webview born hidden (or tiny) never paints
  (WebView2 composition pipeline), and shrinking parked webviews would
  force WhatsApp to re-render on every switch. Position-only moves keep
  switching instant (measured under 350 ms full repaint, switches feel
  immediate).
- A geometry watchdog re-aligns every webview every 2 s (plus resize and
  scale events) so the four corners always hug the client area after any
  DPI or resize dance.
- The drawer is created closed and lazy. Each open DESTROYS and re-opens
  it so it always renders above the account webviews (z-order) with
  fresh state. Closing parks it with the same geometry watchdog path
  (instant).

## Automation flow (host to page and back)

1. Commands: the host evaluates guarded bridge expressions
   `JSON.stringify(function(){ try { if (!window.__zapdudeApiV1 || typeof
   ... !== 'function') return null; ... } catch ... })`; results come
   back via `eval_with_callback`.
2. Degrade, never crash: a missing bridge or moved WhatsApp markup
   becomes a structured API error (`bridge_unavailable`), never a panic.
3. Notification hook: the bridge replaces `window.Notification` and
   hooks page mutations (chat list observer) to surface new message
   events, each tagged with the account id that produced it.
4. Beacon fallback: when the sync eval path dies, the page mirrors
   pending responses into `location.hash = #__zapdude=<payload>` and the
   host poller consumes them.
5. Events: ring buffer of 200 (polled `GET /api/events?since=`),
   broadcast SSE (keep alive 15 s), optional HMAC-SHA256 signed webhook
   (`X-Zapdude-Signature: sha256=...`) delivered with `ureq` v3.

## Multi account model

- Each account runs its own WebView2 profile at
  `%APPDATA%\app.zapzap.desktop\webview\<account-id>` so sessions,
  cookies and storage never mix.
- Deleting an account removes its webview AND wipes its profile folder
  (best effort); the shell falls back to the next account or the
  welcome screen (the welcome screen is the only place with an explicit
  "Add account" button; nothing auto-creates empty accounts).
- Renames are metadata only: ids are stable forever.

## Tray and window behavior

- Closing the main window hides it (app stays alive in the tray).
- Tray menu rebuilds on account add/remove/rename so entries are always
  current; left click on the tray icon shows and focuses the main window.
- "Quit Zapdude" stops the loopback server first, then exits.

## Security posture

- API listens on 127.0.0.1 by default. `exposeNetwork` (LAN) is
  available but off by default and considered a footgun.
- Auth is `X-API-Key` or Bearer; rate limit 60 requests/min; bodies
  capped at 1 MB; webhooks signed HMAC-SHA256 with the API key secret.
- No telemetry, no third-party calls, no global listeners.

## Testing and build

- `cargo test`: unit tests for the API stack (14 green as of this
  writing). The webview shell is verified manually with real window
  captures (PrintWindow + PW_RENDERFULLCONTENT, see
  docs/SESSION-STATUS.md).
- Windows build: `tools\tauri.exe build --bundles nsis` or
  `cargo install tauri-cli --locked` and `cargo tauri build --bundles nsis`.
- Debug bisection: env var `ZAPDUKE_WEBVIEW_OPTS` toggles webview options
  (UA, scripts, data dir, args, permissions) to isolate the cause when
  WebView2 misbehaves.
