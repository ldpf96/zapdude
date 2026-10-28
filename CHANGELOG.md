# Changelog

Notable changes to this project are documented here. The format follows
Keep a Changelog; versioning is SemVer.

## [1.1.0] - 2026-10-28

### Changed
- The whole project (UI, docs, installer, tray, code comments) is now in
  English.
- Honest origin story on the record: the design reference was ZapZap by
  Rafael Tosta, early versions intentionally tried to feel like it, and
  the implementation grew into something substantially different. NOTICE
  and README tell the story, plus the development stack: OpenCode agent
  harness + GLM-5.3 model (by Z.ai), vibe coding style, author verified
  everything on real hardware.
- Installer ships English only.

### Fixed
- (from 1.0.0 line carried forward) App icon: the icon.ico embedded in the
  exe was rebuilt byte for byte from the approved green squircle frames so
  the taskbar, title bar, tray and installer all show the green with the
  black bolt (Windows icon caches may need one Explorer restart).

## [1.0.0] - 2026-10-28

### Added
- Multi account WhatsApp Web desktop client written in Rust (Tauri 2 +
  WebView2). Original project by ldpf96; the design reference was ZapZap
  by Rafael Tosta.
- Single window shell v3 (feature `unstable`): an account rail (76 px),
  one WhatsApp Web webview per account with an isolated WebView2 profile
  (`webview/<id>`), a welcome backdrop and a full screen options panel in
  the same window (the rail logo toggles it).
- Multi account flows: "+" adds an account with no forms and opens its QR
  right away; instant account switching via position-only parking
  (parked at full size outside the window, no re-render, under 350 ms);
  lazy loading so every account creates its webview on first open.
- Premium rail: Zapdude logo, avatars with the bolt (no letters or
  numbers), green ring on the active one, state dot (connected/QR/
  loading), curved dividers, a visible red X when right click arms
  deletion, incremental DOM updates (no flicker). No browser context
  menu on the shell UI.
- System tray: "Show <account>" per account, "Hide all windows", "Quit
  Zapdude". Closing the window hides it to the tray. Single instance by
  mutex.
- Stable Chrome identity (UA 156 + `client_hints_fix.js`) and
  pre-approved permissions for microphone, camera, screen, notifications
  and clipboard so WhatsApp Web calls work.
- Local API for people and agents: REST (`healthz`, `config`, accounts
  CRUD + open/hide, `status`, `open-chat`, `message`, `chats`, `events`,
  `webhook`) + SSE (`/events/stream`), loopback by default,
  `X-API-Key`/Bearer auth, 60 rpm, 1 MB body cap, events tagged per
  account and HMAC-SHA256 signed webhooks (`X-Zapdude-Signature`). The
  shell UI (rail/options/welcome) is served by the API server itself
  (same origin, no IPC).
- Automatic WhatsApp promo cleanup: the "Download WhatsApp for Windows"
  banner on the landing and the download card in the empty chat panel,
  with protected zones (never touches the QR, the chat list or the
  composer).
- Resident page bridge (`api_page_bridge.js`) with
  `eval_with_callback` primary transport and a URL hash beacon
  (`#__zapdude=`) fallback, a 30 ms poller thread per account; deferred
  installs with a per-script idempotency flag.
- Deleting the last account shows the welcome screen with an explicit
  "Add account" button (no empty accounts are auto-created); on a virgin
  install the shell auto-creates "Account 1" like the reference client.
- Persistent config at `%APPDATA%\app.zapzap.desktop\config.json`
  (camelCase, BOM tolerant) with a sanitized `api` block, `prefs` and
  `accounts`.
- Preferences: light/dark theme (shell only), start with Windows (HKCU
  Run key via winreg), start minimized (lives in the tray).
- Call pop-outs: `on_new_window` creates a real window that shares the
  opener's WebView2 environment (same session/cookies).
- Symmetric rail spacing with a "more below" indicator and wheel
  scrolling; account renaming (tray, rail and API see the new name).
- NSIS installer (`tauri build --bundles nsis`, `installMode:
  currentUser`, uninstall hook that wipes the data folder).
- License AGPL-3.0 with a NOTICE telling the origin story (design
  reference ZapZap by Rafael Tosta, authored by ldpf96, AI assisted with
  OpenCode + GLM-5.3).

### Fixed
- WebView2 gray page (two chained causes): webviews born hidden or tiny
  never start their composition pipeline (they are born at full size and
  parked by position), and the old promo sweep hid whole containers
  (rewritten smallest-node-only with supervised collapse).
- Script injection: only the minimum runs at document-start; the bridge
  and the hider install deferred through the beacon poller (one big
  document-start eval reliably breaks WebView2 painting).
- WebView2 `ExecuteScript` arrives double encoded when the value is a
  String: `evaluate` unwraps one more level (data used to fall back to
  defaults silently).
- axum 0.8 nesting at root panics: flat `/api/...` routes.
- `session_state` is honest with 3 states: `detecting` / `scan_qr` /
  `logged_in`, with a 2.5 s cache so the rail answers instantly.
- The API server starts BEFORE the webviews (no more Edge "can't reach
  this page" on open) and the bind retries 40x250 ms (port changes used
  to collide with error 10048).
- Promo buttons whose `textContent` drags the SVG title ("Downloadic-
  download"): `cleanText()` strips `ic-*` tokens before exact matching;
  the deferred installer uses one flag per script so a bridge failure
  never silences the banner cleaner again.
- App icon: `icons/icon.ico` embedded in the exe carried the stale
  teal/white bolt look; rebuilt byte for byte from the approved green/black
  PNG frames (7 sizes) and the UWP tiles re-rendered with the same GDI+
  recipe (`scripts/regen-icons.ps1`, auditor in `tools/dump-ico.ps1`).
- Options drawer close speed: `toggle_drawer` used to wait for the 2 s
  geometry watchdog; `relayout` now runs inside the toggle (parking
  measured at 1 ms).
- Account rename list: the drawer rebuilt the list every 5 s and reset
  the input you were typing in (and stole its focus). Rows are now
  persistent by id and the refresh never touches a focused or dirty
  input; untouched rows still follow server-side renames.
