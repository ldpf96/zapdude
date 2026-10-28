# Session status - Zapdude (Rust)

> Continuity document for AI agents and future sessions. Last updated:
> 2026-10-28 (v1.2.0 shipped).

> **STATE OF THE WORLD**: v1.2.0 is live at
> https://github.com/ldpf96/zapdude/releases/tag/v1.2.0 with CI-built
> artifacts (macOS .dmg aarch64 + x64, Windows NSIS). The app-wide
> theme feature ships in this version. The repo is public, main matches
> the release tag (fdc39dd).

> The app is named **Zapdude** ("dude" lowercase, first letter
> capital: it is exactly "Zapdude", NEVER "ZapDude").
> All project text (UI, docs, comments, release notes, release bodies)
> is in ENGLISH with plain punctuation (no em dashes; explicit user
> requirement). Project copy must read like product text: factual
> statements, normal-length credits (ZapZap by Rafael Tosta as design
> reference, AI tooling named plainly). No confessional phrasing - the
> user explicitly rejected "honest origin story" style wording.

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

- The user runs the installed exe `C:\Users\PC\AppData\Local\Zapdude\
  zapdude.exe` (v1.2.0, verified via `/api/healthz`). The app usually
  lives in the tray (prefs: autostart + startMinimized true).
- The user prefers Spanish in chat; repo text stays English.
- Windows is in dark mode (`AppsUseLightTheme = 0`); the user still
  chose the light token and the in-app theme switches it (now correctly
  through the whole window).

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
4. WebView2 `ExecuteScript` arrives double encoded when the value is a
   String - unwrap twice.
5. WebView2 COM work MUST happen on the main thread: use
   `webview.with_webview(...)` (that is what the theme pinning uses).
6. Debug harness: PrintWindow with `PW_RENDERFULLCONTENT` (2) captures
   DirectComposition content of background windows without stealing
   focus; bisection env var `ZAPDUKE_WEBVIEW_OPTS` remains valid.
7. Check `github.com/settings/tokens` facts before claiming platform
   limits: several past conclusions ("macOS impossible") were wrong
   and got fixed in the same session they were re-verified.

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

1. Linux builds: add an ubuntu matrix row (webkit2gtk-4.1 dependency
   set) to `.github/workflows/release.yml`.
2. macOS: make the chat area follow the in-app theme (WebKit has no
   profile-level color-scheme API like WebView2; would need CDP or CSS
   work), call pop-out windows on macOS (window.open handling without
   WebView2 environment sharing), mic/cam usage descriptions in the
   macOS Info.plist.
3. Mobile (Android/iOS): phase 2 - the multi-webview shell needs a
   redesign (one window = one webview on mobile). The
   `icons/android` and `icons/iOS` assets are already generated.
4. Push events (Notification hook) redesign; passive API works.
5. User live tests on the family Mac (first install + xattr step).

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
