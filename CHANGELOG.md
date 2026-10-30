# Changelog

Notable changes to this project are documented here. The format follows
Keep a Changelog; versioning is SemVer.

## [Unreleased]

### Added
- The drawer's About card gains a **Search updates** button: the light
  tier of the auto-update flow. Every drawer open runs one QUIET check
  against the GitHub Releases API that announces only when a newer tag
  exists; the button itself answers everything ("Searching…", "New
  version X is out - Download", "You are up to date", or the offline
  notice). The running version comes from `/api/healthz` and the
  announcement links straight to the release page. GitHub payload
  fields are shape-checked before they ever touch innerHTML.

## [1.3.1] - 2026-10-31

### Fixed
- Missed-call toasts now wear the SAME clean round avatar the incoming
  ring toast used: the ring surface stores its channel avatar keyed by
  the caller's handle (`window.__zapdudeCallerIcon`), and the
  missed-call hook swaps WhatsApp's own composite icon for it (WhatsApp
  ships a re-cropped photo with the WhatsApp badge stamped on - the
  user called it out: "sale diferentemente ajustada y con el logo de
  WhatsApp"). No ring seen -> the composite icon stays (degrade, never
  crash). Verified live: the missed event's icon is the exact CDN URL
  the ring surface carried, title mapped to the human name, silent.
- WhatsApp Web's v2 empty-state promo card ("Descarga WhatsApp para
  Windows / Obtén funciones adicionales, como llamadas…" with the
  Descargar button) is now hidden by its own semantic testid
  (`intro_panel_v2_title_card`) - the generic banner sweep could never
  collapse it because the bare "Descargar" button text blocks the
  ancestor climb. The functional action-tile row stays.
- The "create an account" welcome screen no longer flashes for a second
  at every startup when accounts exist: the shell boots the welcome
  webview as a pure brand splash (`?splash=1`, logo only), and the full
  picker only renders in the real no-accounts state - where it also
  re-gains its add buttons on the delete-last-account path (it
  re-navigates out of the splash variant before taking the screen).

## [1.3.0] - 2026-10-30

### Added
- Multi-service accounts: each account now carries a `service`
  (`whatsapp` legacy default, `facebook`) that selects the REAL web chat
  the embedded view loads (WhatsApp Web / the Messenger chat surface).
  Facebook login (email + password + 2FA) happens INSIDE the view, the
  same way WhatsApp's QR does. Old configs load unchanged as WhatsApp.
- Facebook receive-side phase 1 over the local API: `service` field on
  `/api/accounts`, `/api/status` and on every incoming event; session
  states `logged_in` / `needs_login` (login form detected) / `detecting`;
  `status`, `chats` and the event stream are service-agnostic, while the
  WhatsApp-only operations (`message`, `open-chat`, calls) answer a
  structured `unsupported_for_service` on other accounts to keep the
  API contract clean.
- A new adaptive page bridge for Facebook (`facebook_bridge.js`) under
  the SAME `window.__zapdudeApiV1` contract: honest degraded reads
  (multi-strategy selectors, structured results), a document.title
  unread watcher shipping `title_badge` events, and every entry point
  guarded so markup drift can never break an endpoint.
- The service picker: clicking the rail's "+" opens a horizontal brand
  strip anchored beside the tile (WhatsApp / Messenger logos, real
  simple-icons CC0 glyphs); it follows the tile, and closes on pick,
  Esc, outside click or cursor exit.
- Incoming calls surface as the SAME branded native toast the message
  notifications use (caller identity via the semantic testid first,
  avatar pass-through, per-node dedupe), while the caller's own
  "Calling.../Llamando..." surface stays SILENT (the caller is already
  in the call; the real capture had produced one bogus toast) and toast
  titles arrive clean (WhatsApp's "~" presence glyph stripped). The
  in-page call prompt keeps working with a cosmetic brand-styling pass
  (rounded card, soft shadow) anchored on its stable testids - CSS only,
  buttons keep their native pill shape, the stylesheet self-degrades on
  markup drift.
- Uniform toast law: every native toast renders at THE SAME SIZE at
  birth - the icon slot is always occupied (the contact avatar when the
  payload carries one, the app icon as universal fallback otherwise),
  titles clamp to one line and bodies to exactly two (short bodies get
  padded with an invisible non-breaking space), so message, call and
  status toasts read as one family.
- Missed-call notifications are fully branded: WhatsApp ships the caller
  avatar as a huge inline data URL which the flat 4096-char length gate
  used to reject silently, so the toast rendered bare; the gate now only
  applies to remote URLs and inline payloads decode up to 300 KB. The
  missed-call toast also carries the CALLER's human identity: the ring
  relay learns the handle -> display-name pair at ring time (WhatsApp
  titles the missed Notification with the bare handle) and the page hook
  swaps it in.
- A service-worker guard stops the SQUARE notification: WhatsApp's own
  worker (web.whatsapp.com/sw.js) raises ring and push notifications
  from its own realm via `registration.showNotification` - an un-stylable
  OS bubble the branded pipeline can never touch (the user saw it
  stacking behind the branded toast). The guard intercepts
  `navigator.serviceWorker.register` (the original promise still
  resolves so the page starts normally) and sweeps existing
  registrations every two seconds; the page-level capture keeps feeding
  the branded pipeline.
- A stable-backup practice: `tools/backup-stable.ps1` snapshots the
  verified dev build (exe + symbols + full source zip + verification
  manifest) into `backups/stable-dev-<timestamp>/`, keeping the newest
  twelve.

### Changed
- Toast anatomy: each notification reads contact (bold) + message, with
  the contact avatar as a small round icon at the LEFT; the redundant
  "Zapdude" title and the raw `[account-1]` tag are gone, and clicking
  opens the account - and now the chat - that produced the notification
  (the hook's stored onclick handlers get re-dispatched).
- Rail: real service glyphs with brand colors per account, honest state
  dot (green = connected, yellow = waiting for QR/sign in, red = page
  problem, grey = detecting) backed by stale-while-revalidate session
  caching so the color stops rotating, and the "+" pinned near the
  rail's bottom edge.
- The attribution file now credits the third-party assets in use
  (simple-icons CC0 icons for the embedded services) and carries the
  trademark wording for WhatsApp, Messenger and Facebook.
- The "download the desktop app" promo hider now also matches the
  short Spanish "descarga la app" family of texts (plus `WhatsApp
  Desktop` variants), keeping its protected-zone guardrails.
- `/api/call` survives WhatsApp's call-button label drift: the click
  matches exact labels first ("Llamada", "Llamada de voz", "Voice call",
  "Llamar") and falls back to the first call-ish label that is not the
  video button (live diagnostics caught the plain Spanish "Llamada"
  label leaving the endpoint answering `call_button_not_found`); the
  failure payload now lists observed labels for the next drift.

## [1.2.1] - 2026-10-29

### Added
- Branded notifications, ONE uniform surface: WhatsApp Web's page-level
  `Notification` calls are captured by a deferred page hook and raised as
  native Windows toasts through the registered Zapdude identity (AUMID
  `app.zapzap.desktop`), with the contact avatar when the payload carries
  one (both CDN URLs and inline data:image icons) and click-to-restore.
  One look for every state - window visible, hidden or in the tray. The
  toasts double as push events on the API bus (`SSE`, polling, webhooks).
  (An earlier dual design - in-app overlay when visible, native when
  hidden - read as "different notifications per account" in real use and
  was replaced by the uniform route.)
- Call automation: `POST /api/call` (open a chat and click the voice/video
  call entry point, auto-opening the account webview when needed) and
  `POST /api/call/end` (click the hang-up control of the active call
  surface). Popup lifecycle is now observable: `popup_opened` and
  `popup_closed` events land on the API bus, and the account page gets an
  idempotent recovery prompt (`callRecover`) the moment its popup dies
  without WhatsApp's own end-call flow.
- Chat-opening automation goes through the search drawer (with retry beats
  and a robust pick for own-number and unsaved-number results). The old
  `?send` deep link trapped cold-started webviews on an unresolvable
  redirect (about:blank) and is gone from the flow.
- Debug harness: `ZAPDUDE_DEBUG_EVAL=1` unlocks `POST /api/_debug/eval`
  (live JS evaluation in account webviews), `GET /api/_debug/windows`,
  `POST /api/_debug/windows/{label}/close` and `POST /api/_debug/emit`
  (synthetic bus events). All key-gated and loopback-only, never rate
  limited (so live sessions can poll), production-safe by default.
- Second-launch behavior: when Zapdude already lives in the tray, a second
  activation (desktop shortcut, Start Menu, exe double click) now pings the
  running instance through `POST /api/ui/show`, which brings the window
  back - no more dead clicks. Autostart migration: the Run key entry now
  carries `/startminimized`, so only auto-launched instances stay hidden;
  manual clicks always open the window.

### Fixed
- Media download: the promo hider no longer matches bare "Download"/
  "Descargar" buttons outside the logged-out landing flow. The media
  viewer overlay lives outside the protected chat zone and carries a real
  Download button; the oversweep erased it (the download option that
  vanished). Dialogs and menus are now protected zones as well.
- Chat list scraping: WhatsApp moved the list to `data-testid="list-item-N"`
  grid rows; the scraper follows the new markup (plus the dedicated unread
  badge test id) and reports `unread_count` per chat.
- Recreated account webviews kept the original page bridge, so every
  automation call after a close/re-open cycle timed out against a dead
  webview. Bridges are now replaced on recreation and cancelled (poller
  exits) when the webview goes away.
- Composer readiness tolerates the deferred-bridge installation gap after
  page navigation/reloads (wait state, not failure).

## [1.2.0] - 2026-10-28

### Added
- App-wide light/dark theme: account views pin the theme color scheme on
  their WebView2 profiles (`prefers-color-scheme`) at creation and on
  preference changes, so WhatsApp Web follows the in-app theme live -
  no reload, no session round trip. Native caption bars follow too
  (`Window::set_theme`, applied to the main window and call pop-outs).
- macOS support: first `.dmg` builds (Apple Silicon + Intel) via
  TypeScript-free CI on GitHub Actions (macOS 14+, unsigned, distributed
  directly). `icon.icns` added to the bundle list and the macOS minimum
  system version pinned to 14.0 (multiwebview needs the private APIs and
  only works there).
- Release workflow `.github/workflows/release.yml`: tagged builds for
  macOS (aarch64 + x86_64) and Windows NSIS from the same tag.

## [1.1.0] - 2026-10-28

### Changed
- The whole project (UI, docs, installer, tray, code comments) is now in
  English.
- Project origin and credits consolidated in NOTICE.md and README.md:
  the design reference (ZapZap by Rafael Tosta) and the development
  tooling (OpenCode agent harness + GLM-5.3 model by Z.ai, with every
  change verified on real hardware by the author) are documented there.
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
- License AGPL-3.0 with a NOTICE crediting the design reference (ZapZap
  by Rafael Tosta), the author (ldpf96) and the AI-assisted tooling
  (OpenCode + GLM-5.3).

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
