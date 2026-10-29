# Session status - Zapdude (Rust)

> Continuity document for AI agents and future sessions. Last updated:
> 2026-10-29 (working tree past v1.2.0, pre-release; features below need
> a version bump once the user signs off).

> **STATE OF THE WORLD**: v1.2.1 is live at
> https://github.com/ldpf96/zapdude/releases/tag/v1.2.1 with CI-built
> artifacts (macOS .dmg aarch64 + x64 with macOS 13 minimum and the
> privacy plist keys; Windows NSIS). v1.2.0 remains live for older
> installs. Download site resolves the latest release automatically.
> The release body documents the Ventura caveat and the 1.2.1 highlights. The app is named **Zapdude** ("dude" lowercase, first letter
> capital). Repo text is ENGLISH; chat with the user is Spanish.

## What changed in v1.2.1 (working epoch, knowledge for future sessions)

- **Branded notifications (dual).** Page-level `Notification` on WhatsApp
  webviews is captured by a deferred hook (`scripts/notification_hook.js`,
  flag `__zapdudeNotifyV1`; the DOCUMENT-START variant of this hook once
  killed startup - the deferred slot is the proven safe install point).
  The stub ships notification events through the beacon transport into
  the bus (kind "incoming", payload kind "notification", which kept the
  documented "incoming" contract; the ORIGINAL constructor is never
  called, so WebView2 never shows its web-style toast). `src/notifier.rs`
  routes them: main window visible -> the `toast-overlay` child webview
  (branded stack, `ui/toast.html`, polling `/api/ui/toasts?since=N`,
  click -> `/api/ui/toast/action` -> account focus + page-level onclick
  re-dispatch through `window.__zapdudeFireNotificationClick(tag)`, idle
  -> `/api/ui/toast/release`); main window hidden -> native Windows toast
  via `tauri-winrt-notification` direct (AUMID `app.zapzap.desktop` =
  the registered Start Menu identity; the notification plugin's default
  fell back to the "powershell" attribution the user flagged) with the
  contact avatar cached in `<data>/toast-icons/`.
- **Call surface observations (live, key facts).** As of the observed
  build, placing a call from the webview did NOT open a popup for our
  automated clicks (no window.open observed at any sample; 900 ms
  samples over windows). The call log lands as in-chat bubbles
  (`voip_icce_bubble_1on1_call`, `call-log-system-message`,
  `call-log-icon-chip`) and the callee side registered a real ringing +
  missed-call notification (captured live on the callee account, hook
  working). The user reports the manual call UI as an in-page draggable
  square ("cuadradito"); the passive recorder `scripts/call_anchor.js`
  (flag `__zapdudeCallAnchor`) ships a `call_surface` event through the
  bridge whenever a call surface mounts - the next natural user call
  will give the exact markup to anchor it. Popup windows still exist in
  the app (real windows, `popup-*` labels, shared environment, native
  drag): paths that DO call window.open (media popups and older call
  flows) are handled by `popup_window()` with lifecycle tracking
  (POPUP_INDEX + Destroyed -> `popup_opened`/`popup_closed` events +
  `callRecover` reconciliation prompt on the account page; the recovery
  clicks the "volviendo a la llamada/return to call" control when
  present).
- **Call/chat automation API.** `POST /api/call` opens the chat (drawer
  search flow, auto-opens the account webview when missing) and clicks
  the voice/video header button; `POST /api/call/end` clicks hang-up.
  Chat opening rewrote: `?send?phone=` deep links DROP cold webviews to
  about:blank (unresolvable redirect); the drawer flow
  (`openChatByPhone` -> typing beat -> `pickPhoneResult` -> composer)
  works from any state and matches unsaved numbers AND the own-number
  self-chat row (`message-yourself-row`). Verified live end to end:
  SELF-SEND on account-1 and CROSS-SEND account-2 -> +51 number both
  delivered real WhatsApp messages.
- **Second launch shows the window.** Single-instance mutex now signals
  the first instance through `POST /api/ui/show` (config-file port,
  retries). Autostart Run key value became `"<exe>" /startminimized`;
  only auto-launched processes honor the start-minimized pref: manual
  clicks always open the window. Verified: the Run key migration happens
  best-effort at every startup (prefs on) and /api/ui/show works live.
- **Bridge lifecycle fixed.** `PAGE_BRIDGES`: closing and re-creating an
  account webview KEPT the first bridge (dead target, eval timeouts
  forever). New `page_ops::replace`/`remove`; the beacon poller exits
  when the bridge is cancelled (`cancelled` flag on PageBridge).
  Verified: account-2 close/open cycle recovers.
- **Chat list scraper follows the 2025-10 markup.** Rows:
  `#pane-side [data-testid^="list-item"]`, title `span[title]` first,
  snippet `cell-frame-secondary span[title]`, unread badge
  `[data-testid="icon-unread-count"]` (digits from its aria-label,
  locale-independent). Verified live with real chats (288/311 unread
  group).
- **Debug harness.** Under `ZAPDUDE_DEBUG_EVAL=1`:
  `POST /api/_debug/eval` (raw expression eval via the page bridge with
  the defensive wrapper), `GET /api/_debug/windows` (windows + webviews),
  `POST /api/_debug/windows/{label}/close` (shell core refused; account
  labels cancel their bridge first), `POST /api/_debug/emit` (synthetic
  bus events for SSE/webhook/toast testing). Debug auth skips the rate
  limiter (monitors poll fast) but keeps the API key + enabled checks.
  Helper scripts live in `tools/` (see debug-eval.ps1 for evaluation;
  sse-listen.cmd + webhook-receiver.ps1 = the verification battery).
- **Promo hider hardened.** The generic sweep abandoned the bare
  "Download"/"Descargar" exact-text match (it erased the REAL Download
  button of the in-page media viewer overlay, the "download option
  disappears" report); dialogs/menus are protected zones now. The
  landing flow keeps its stricter copy+card logic.

## What changed in v1.2.0 (knowledge for future sessions)

- **App-wide theme.** Each account view's WebView2 profile pins
  `ICoreWebView2Profile::PreferredColorScheme` (LIGHT/DARK) to the shell
  theme: applied at view creation (`ensure_account_webview`) and on
  preference changes (`post_prefs`). WhatsApp Web follows it live (it
  listens to `matchMedia` changes) without reloading or losing the
  session. Native caption bars follow via `Window::set_theme` on the
  main window and pop-outs. Implemented in `window.rs`
  (`apply_theme_to_message_views`, `apply_window_theme`,
  `apply_theme_everywhere`), hooked in `src/api/server.rs` `post_prefs`.
  New dependencies (windows-only): `webview2-com = "0.39"`,
  `windows-core = "0.62"` (same version family tauri/wry already
  resolved). Verified live via PrintWindow captures: light boot, dark
  toggle, light back, session intact (`session_state: logged_in`
  throughout).
- **macOS support.** Tauri 2 multi-webview DOES run on macOS 14+ via
  private APIs (unstable feature), which rules out App Store
  distribution but works for direct distribution. `tauri.conf.json`
  pins `bundle.macOS.minimumSystemVersion: "14.0"`, `icons/icon.icns`
  (already generated) is part of the icon list. Read the docs before
  repeating the old "macOS discarded" claim - that is OBSOLETE.
- **CI**: `.github/workflows/release.yml` builds from a tag
  (+ workflow_dispatch): macOS aarch64/x86_64 (--bundles app,dmg) and
  Windows (--bundles nsis). tauri-apps/tauri-action@v0, GITHUB_TOKEN,
  releaseDraft: false. Runtime observed ~7.5 minutes, 3/3 green.
- **Release copy bug worth remembering**: PowerShell 5.1 `Get-Content`
  wraps the string with PSObject metadata (PSPath, PSParentPath...)
  and `ConvertTo-Json` serializes the wrapper. Use
  `[System.IO.File]::ReadAllText()`, then `ConvertTo-Json`, then send
  `[Text.Encoding]::UTF8.GetBytes($json)` as the request body. Both
  release bodies (v1.1.0 title "Full English release", v1.2.0) were
  PATCHed clean through the API after this bug.

## Current user state

- AFTER the v1.2.1 workflow: the user's PC still runs the DEV build
  (debug binary at `target\debug\zapdude.exe`, started through
  `tools\run-dev.cmd`, console captured to `zapdude-console.log`, env
  `ZAPDUDE_DEBUG_EVAL=1`). When that PC session ends, the dev instance
  dies and NOTHING runs unless the user installs v1.2.1
  (`Zapdude_1.2.1_x64-setup.exe` from the release) - until then a
  manual double click of the OLD installed exe (v1.2.0 at
  `C:\Users\PC\AppData\Local\Zapdude\zapdude.exe`) brings back the
  previous behavior. Recommend installing v1.2.1 first thing next
  session (single instance mutex: quit the running instance first,
  install, relaunch).
- The loopback API stays ENABLED in
  `%APPDATA%\app.zapzap.desktop\config.json` (key unchanged, port
  29977) after the day's battery; the user knows it runs.
- All v1.2.1 features were verified against the DEV build with real
  traffic and real sends (self-send, cross-send account-2 -> +51). The
  installed v1.2.0 exe will gain them only through the v1.2.1
  installer.
- The family Mac project (sister-in-law, other user): macOS **13.7.8
  Ventura, Intel**. The `Zapdude_1.2.1_x64.dmg` was handed over (user's
  downloads folder, SHA-256 verified against the release asset). The
  Ventura run is the pending live test of the macOS 13 minimum gamble:
  awaiting their report (window up, QR, chats readable). If Ventura
  misbehaves: plan B is redesigning the shell for macOS (one window
  per account) or asking for a macOS 14 update.
- The user prefers Spanish in chat; repo text stays English.
- Windows is in dark mode; the in-app theme follows its preference.

## Requirements from the user (stable, in importance order)

1. Multi account is CRITICAL: all WhatsApp accounts in one app.
2. Premium human interface (green brand, no browser chrome).
3. His name (ldpf96) on the credits.
4. Local loopback API for humans and AI agents (REST + SSE + signed
   webhooks).
5. Calls/video/screen sharing (Chrome 156 JS identity).
6. No terminal on launch, NSIS installer, clean uninstall.
7. "Never break": failures degrade, never crash.
8. Publish on GitHub with professional release copy.
9. English everywhere in the repo; Spanish in chat only.

## Paths

- Project: `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs`
- Runtime config: `%APPDATA%\app.zapzap.desktop\config.json`
  (identifier stays `app.zapzap.desktop` to preserve user data).
- WebView2 profiles: `%APPDATA%\app.zapzap.desktop\webview\<account-id>`
- ZapZap reference copy (READ ONLY): `C:\Users\PC\Desktop\Projects\
  Zapdude\zapzap-main`

## Architecture snapshot (see README + architecture.md)

- ONE window "main" (Tauri 2 `unstable` multi-webview): rail webview +
  welcome webview + N account webviews + a lazy full-screen drawer
  webview. Active content webview sits at `(RAIL_WIDTH, 0)`; idle
  content webviews park at `x=-24000` logical at FULL SIZE (never
  hidden). Geometry watchdog re-aligns every 2 s and on events.
- The loopback API server (axum) SERVES the shell pages: same-origin
  fetches, no Tauri IPC for the UI.
- Tray: per account "Show", "Hide all", "Quit". Startup order:
  single instance mutex, server, shell, first account.
- Account/popup labels: `account-<id>` / `popup-<account>-<n>`.

## Lessons that must not regress

1. Webviews born hidden or tiny stay gray FOREVER: born at full
   content size, then parked by position only (never hide()).
2. Custom `additional_browser_args` + Chrome UA on child webviews
   breaks WebView2 painting (do not ship custom args).
3. Keep document-start scripts minimal; install the rest deferred
   through the beacon poller with per-script idempotency flags.
   (The Notification hook in particular MUST stay deferred: at
   document-start it once killed WhatsApp startup.)
4. WebView2 `ExecuteScript` arrives double encoded when the value is a
   String - unwrap twice. Also: a returned PROMISE resolves fine, but
   plain JS objects jumped straight from async code inside one eval do
   not ship - keep browser-op scripts synchronous, pace with the host.
5. WebView2 COM work MUST happen on the main thread: use
   `webview.with_webview(...)` (that is what the theme pinning uses).
6. Debug harness: PrintWindow with `PW_RENDERFULLCONTENT` (2) captures
   DirectComposition content of background windows without stealing
   focus; bisection env var `ZAPDUKE_WEBVIEW_OPTS` remains valid.
7. Check `github.com/settings/tokens` facts before claiming platform
   limits: several past conclusions ("macOS impossible") were wrong
   and got fixed in the same session they were re-verified.
8. Bridge ops receive POSITIONAL arguments as rendered by
   `bridge_expression` - passing an object-shaped args string makes the
   op see "[object Object]" (the "phone_too_short" live bug).
9. The `?send?phone=` WhatsApp deep link can drop a cold webview onto
   about:blank; the drawer search flow is the reliable opener.
10. API rate limit (60/min) is shared with live debug monitors - the
    debug auth path skips it, keep debug polling on that path.
11. Crates with unguarded target deps go in
    `[target.'cfg(windows)'.dependencies]`, not `[dependencies]`
    (tauri-winrt-notification drags windows 0.62 -> windows-future 0.3.2
    into the macOS graph and both mac CI jobs failed to compile; the
    gated move restored the green build).
12. The user's family Mac runs macOS 13.7 (Ventura): the v1.2.1 build
    opened LSMinimumSystemVersion to 13.0 and shipped usage-description
    plist keys for mic/camera/screen. State on Ventura is EXPERIMENTAL
    (the multi-webview engine is verified on macOS 14+); the release body
    and Pending item 5 carry this.

## Git / GitHub / auth state

- Remote: `https://ldpf96@github.com/ldpf96/zapdude.git` (username-
  anchored).
- Two credentials exist locally: Git Credential Manager OAuth
  (`gho_...`, scopes `gist, read:org, repo` - NO workflow scope) and a
  classic PAT the user pasted in chat (scopes `repo, workflow`). The
  PAT was used inline (URL-embedded) for the v1.2.0 push and was NOT
  stored in the keyring; recommended the user revoke it after use.
- Pushes that CREATE/UPDATE workflow files require the `workflow`
  scope (any token lacking it gets rejected at the remote). Code/docs
  pushes work with the stored OAuth token.
- GitHub CLI exists at `C:\Program Files\GitHub CLI\gh.exe` but is not
  on PATH; invoke with the full path if needed.
- The tag push trigger of the release workflow did not fire for
  v1.2.0 (new workflow + same-session push); workflow_dispatch through
  the API worked. Keep an eye on it for the next tag.

## Pending (next candidates, user decides)

1. Install v1.2.1 on the user's Windows PC FIRST (replace the v1.2.0
   install; remember the single instance mutex). Everything verified
   today ran on the dev build, not the installed one.
2. Family Mac report: did the v1.2.1 x64 DMG open and pair on Ventura
   13.7.8? (lsmin 13.0 experiment; the release body carries the caveat -
   feed the outcome into docs and, if needed, plan the macOS shell
   redesign.)
3. Call surface anchor phase 2: the passive recorder
   (`scripts/call_anchor.js`) ships `call_surface` events whenever a
   real call surface mounts. With the user's next natural call, take
   that data and pin the in-page call panel (no drag inside the app
   box). The popup path is already recovered automatically
   (`callRecover` clicked the pill live; the reload failsafe exists but
   remained unused so far).
4. `own_number`: a first-class endpoint for each account's own number
   (DOM does not expose it; candidates: WAM state, settings deep-dive).
   The cross-send direction account-1 -> account-2 still needs the +1
   callee number the user has not shared yet.
5. Push-event coverage from the SERVICE WORKER path (hidden-window
   notifications may only fire through sw.js; the current capture covers
   page-level Notification calls). No COM interception attempted: the
   native toast fallback covers the hidden case already.
6. macOS toast path (NSUserNotification) so Mac builds float avatars like
   Windows does now; also call pop-out windows on macOS (window.open
   handling without WebView2 environment sharing).
7. Linux builds: add an ubuntu matrix row (webkit2gtk-4.1 dependency
   set) to `.github/workflows/release.yml`.
8. User-side checks they agreed to feel: close-to-tray then double
   click the installed icon (window must come back), and a media
   download sanity pass (the option must stay visible).

## Useful commands

```powershell
cd C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
cargo test
cargo build --release
Get-Process -Name zapdude | Stop-Process -Force   # before rebuilding
curl http://127.0.0.1:29977/api/healthz
curl http://127.0.0.1:29977/api/accounts
```

## Environment notes

- PowerShell: cargo/tauri write to stderr (cosmetic
  NativeCommandError noise; judge by content and exit code).
- `Start-Process` needs absolute paths.
- git: `C:\Program Files\Git\bin\git.exe` (PATH may not be refreshed).
- The user speaks Spanish; answer in Spanish in chat, but the REPO
  TEXT stays English per his standing decision.
