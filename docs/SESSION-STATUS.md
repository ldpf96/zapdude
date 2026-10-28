# Session status - Zapdude (Rust)

> Continuity document for AI agents and future sessions. Last updated:
> 2026-10-28 (evening, v8).

> **Next immediate step**: the user is LIVE TESTING the newest install
> (binary 2026-10-28, English UI v1.1.0). Verification pending from the
> user: green icon in tray/taskbar after the icon cache purge (already
> done once), real call pop-out test, scanning a real QR. CI for Linux
> builds pending a run decision (see Round 9).

> The app is named **Zapdude** ("dude" lowercase, first letter
> capital: it is exactly "Zapdude", NEVER "ZapDude").
> The repo is now PUBLIC as **https://github.com/ldpf96/zapdude** with a
> release carrying the NSIS installer. The whole project text (UI, docs,
> comments, release notes) is in ENGLISH, no em dashes or fancy AI
> punctuation anywhere (explicit user requirement).
> Attribution/origin story (explicit): the design and behavior reference
> was ZapZap by Rafael Tosta (GitHub rafatosta/zapzap); early versions
> deliberately tried to FEEL like ZapZap, and the implementation then
> grew into something SUBSTANTIALLY DIFFERENT on a different stack (Rust
> + Tauri/WebView2 vs Python + GTK). No ZapZap code text ships here.
> The user decided on AGPL-3.0-or-later (LICENSE is the verbatim AGPL
> 3.0 text). Development credit on the record: the author drove the
> project with the OpenCode agent harness + the GLM-5.3 model (by Z.ai)
> in a vibe coding style. Author: ldpf96 (GitHub user id 19356038).
> Installer on record: `target\release\bundle\nsis`\
> `Zapdude_1.1.0_x64-setup.exe`; installed binary sits at
> `C:\Users\PC\AppData\Local\Zapdude\zapdude.exe`.

## Current user state (v8)

- The user has 6 accounts (account-1..6) labeled "Cuenta 1".."Cuenta 6"
  (their own renames via the old flow; fresh DEFAULT labels are "Account
  N" now). Fresh installs use "Account 1".
- The app window usually sits hidden in the tray (the user tested "start
  minimized"). Windows icon caches were purged once, so taskbar/tray
  show the green/black artwork.
- The user is also working with OTHER tools (CLI sessions for a second
  project) on this same PC, so shell tests avoid stealing focus and the
  app is usually shown with SW_SHOWNOACTIVATE when needed.

## What this project is

**Zapdude** = WhatsApp Web multi-account desktop client + local API,
written in Rust for Windows (Tauri 2, WebView2, multi-webview unstable
feature). Original code by ldpf96, AGPL-3.0-or-later (LICENSE holds the
verbatim AGPL 3.0 text). The design reference was the ZapZap concept;
early steps tried to mirror its behavior, the final implementation is
substantially different. Publishing story is in NOTICES and README (in
English, honest, credit to Rafael Tosta for the original concept).

Requirements from the user, in importance order:
1. Multi account is CRITICAL: all WhatsApp accounts in one app. A left
   rail, "+" adds an account with NO FORMS, QR shows immediately.
2. A premium human interface (the user loves the green brand).
3. The user wants HIS NAME (ldpf96) on the credits.
4. A local API in loopback for humans and AI agents (REST + SSE +
   signed webhook).
5. Calls/video/screen sharing working (creds: Chrome 156 JS identity).
6. No terminal on launch, Windows NSIS installer, full uninstall.
7. "Never break": errors degrade, they never crash the app.
8. Publish on GitHub (done: repo ldpf96/zapdude + release v1.0.0; next
   release will be v1.1.0 with the English pass).
9. Everything in English (UI + docs) with ZERO em dashes or AI-style
   punctuation; honest origin story in the repo.

## Paths

- Project: `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs`
- Runtime config: `%APPDATA%\app.zapzap.desktop\config.json`
- WebView2 profiles: `%APPDATA%\app.zapzap.desktop\webview\<account-id>`
- Audit of the installed exe icon: ExtractIconEx on
  `C:\Users\PC\AppData\Local\Zapdude\zapdude.exe` (green/black, see
  Round 8).
- Original ZapZap reference copy (READ ONLY, do not modify, it is
  referenced as documentation for behavioral inspiration):
  `C:\Users\PC\Desktop\Projects\Zapdude\zapzap-main`
- Prebuilt tauri CLI: `zapdude-rs\tools\tauri.exe` (gitignored, binary).
- GitHub CLI: `C:\Program Files\GitHub CLI\gh.exe` (installed via
  winget, ldpf96 authenticated in the Windows keyring; `gh auth
  setup-git` already ran, plain git push works).

## Architecture snapshot (see README + architecture.md for prose)

- ONE window "main" (Tauri 2 `unstable` multi-webview): rail webview +
  welcome webview + N account webviews + a lazy full-screen drawer
  webview.
- The ACTIVE content webview sits at `(RAIL_WIDTH, 0)`; every idle
  content webview is parked at `x=-24000` logical at FULL SIZE (never
  hidden), so switching is a position-only move and WhatsApp Web never
  re-renders.
- Window states: `ACTIVE`, `DRAWER_OPEN`; a geometry watchdog re-aligns
  every 2 s and on resize/scale/focus.
- The API server (axum, loopback) serves the shell UI itself: same
  origin fetches, no Tauri IPC for the UI.
- Tray: per account "Show", "Hide all windows", "Quit Zapdude" (rebuilt
  on account changes). Left click on the tray icon restores the window.
- Startup order: single instance mutex, loopback server (bind retries
  40x250 ms), shell, first account (or "Account 1" on virgin install).

## The "gray page" lessons (do not regress these)

1. A webview created hidden or born tiny stays gray FOREVER: children
   must be born at full content size and then parked by position.
2. Passing custom `additional_browser_args` in children together with a
   custom Chrome UA reliably broke WebView2 painting. No custom args in
   production code.
3. A single huge document-start script also broke painting; keep
   document-start minimal (client hints only) and install the rest
   later through the deferred poller with per-script idempotency flags
   (`__zapdudeApiV1` for the bridge, `__zapdudeHidePromos` for the
   banner hider).
4. `ExecuteScript` arrives double encoded when the value is a String
   (unpack twice).

## Debug harness (reusable)

- Real window captures: PrintWindow + `PW_RENDERFULLCONTENT=2` saved to
  `C:\temp\*.png` (DirectComposition content only paints through that
  flag).
- Option bisection: env var `ZAPDUKE_WEBVIEW_OPTS` (comma list) toggles
  ua|scripts|data|args|perms and isolates what breaks.
- Icon ground truth: `ExtractIconEx` on the installed exe renders the
  exact icon the OS draws (title bar, taskbar, tray) and lets you pixel
  sample without touching the screen.
- Window state probing: EnumWindows + GetWindowRect + IsWindowVisible
  on the Zapdude-named top level (child webviews report as
  `WRY_WEBVIEW`, host chrome as `Chrome_WidgetWin_*`).
- Close-speed measurement: poll child `WRY_WEBVIEW` rects around a
  POST `/api/ui/drawer` toggle and watch a rect move from the content
  slot to parked x (<-10000). Measured 1 ms after the Round 7 fix.
- IMPORTANT: when the user is actively using the screen, show the app
  with SW_SHOWNOACTIVATE (never SetForegroundWindow) and restore the
  previous visibility afterwards.

## Round history (short version, details in git log)

- R1: full Rust rewrite chosen (was Electron, then Tauri).
- R2: single window shell, rail premium, delete by right click arming
  (no timed auto-disarm), instant list_accounts via session_state cache.
- R3: drawer options panel in the same window, no rail footer, instant
  switches (position-only parking), no rail flicker (diff renderer).
- R4: full screen options, lazy account loading, visible delete X,
  rename to "Zapdude", repo cleanup for publish.
- R5: theme toggle (light/dark shell only), anclaje fixes (geometry
  watchdog), tray fix (show window first, rebuild menu, left click),
  autostart + start minimized (winreg Run key), icon green generated,
  call pop-outs sharing the opener environment, symmetric rail, account
  rename in the drawer + API.
- R6: user uninstalled for a virgin test (NSIS hook wipes config+
  profiles), half/half theme fix (drawer rebuilt on each open, rail+
  welcome reload on prefs change), rail duplicate draw fix, profile
  wipe on account delete.
- R7: icon.ico in the exe was STALE (teal/white bolt) while the PNGs
  were green; rebuilt ico byte-for-byte from approved PNG frames and
  re-rendered 512/UWP tiles (`scripts/regen-icons.ps1`, auditor
  `tools/dump-ico.ps1`), verified with ExtractIconEx on the live exe.
  Drawer close speed fixed (relayout inside toggle, measured 1 ms).
  Rename UX fixed (persistent rows by id, refresh never touches a
  focused or dirty input, untouched rows merge server renames; E2E
  verified in a real browser against the live server). Silent NSIS
  reinstall flow solid.
- R8: full original-project identity (AGPL-3.0, author ldpf96,
  X-Zapdude-Signature, Zapdude-API/1 UA, `__zapdudeApiV1` flag,
  comments/docs updated; config identifier `app.zapzap.desktop` stays
  to preserve user data). Published to GitHub (ldpf96/zapdude with
  release v1.0.0 carrying the NSIS). Windows icon cache explained the
  "still gray" report; purge + Explorer restart fixed it; verified with
  a taskbar crop screenshot (green with black bolt, running indicator).
- R9 (this round): full English pass (UI, docs, comments, tray,
  installer languages, default labels "Account N"), em dash purge,
  honest attribution rewrite in NOTICE/README/About (design reference
  ZapZap, implementation substantially different, AI assistance named:
  OpenCode + GLM-5.3 vibe coding), version bump to 1.1.0, NSIS English
  only, CI attempt for Linux/macOS builds.

## Verification state (latest evidence)

- Tests 14/14 green at every handoff.
- Published: repo https://github.com/ldpf96/zapdude (public, GitHub
  detected AGPL-3.0), release v1.0.0 with installer asset.
- Icon: green/black on exe (ExtractIconEx sampling), window (WM_GETICON
  sampling), taskbar button (screenshot crop), tray (same icon source).
- Drawer: instant close (1 ms parking), English UI, rename list E2E
  (typing survives refresh ticks, external renames do not clobber edits,
  save persists, blur merges).
- Multi account: 6 accounts live, delete flow with profile wipe, lazy
  webviews, instant switches.

## Pending

- CI for Linux builds (GitHub Actions): attempt, and if it needs
  unusual effort, DROP it per user instruction (macOS is NOT viable for
  this shell: Tauri multi-webview is Windows/Linux only, so the macOS
  build is discarded without a CI run).
- User live tests: scan a real QR, receive a call and pop it out,
  confirm the green icon in taskbar and tray after a restart.
- Redesign push events (Notifications hook is currently partial; the
  API works in passive mode: status/chats/send/open-chat).
- Keep the release notes and CHANGELOG in English going forward.
- The rail icon set (calls/favorites/etc. original-style rail icons)
  stays parked until the user asks.

## Useful commands

```powershell
cd C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
cargo test
.\tools\tauri.exe build --bundles nsis   # installer at target\release\bundle\nsis

# Kill running instances before rebuild (otherwise os error 5)
Get-Process -Name zapdude | Stop-Process -Force

# Clear the bisection env var BEFORE production runs
Remove-Item Env:ZAPDUKE_WEBVIEW_OPTS -ErrorAction SilentlyContinue

# Smoke the API
curl http://127.0.0.1:29977/api/healthz
curl http://127.0.0.1:29977/api/accounts
curl -X POST http://127.0.0.1:29977/api/accounts   # add account
```

## Environment notes

- PowerShell: cargo/tauri write to stderr, PowerShell raises cosmetic
  NativeCommandError; wrap with `cmd /c` and check `$LASTEXITCODE`.
- The same PowerShell shell PERSISTS env vars between commands; always
  clear `ZAPDUKE_WEBVIEW_OPTS` before production runs.
- `curl.exe` (not the PS alias). For POST bodies prefer `--data-binary @file`.
- `Start-Process` needs ABSOLUTE paths (relative fails with "file not
  found" even with the right CWD).
- UTF-8 .ps1: NEVER use em dashes or non-ASCII inside PowerShell string
  literals (parser breaks); keep comments plain ASCII.
- git: `C:\Program Files\Git\bin\git.exe` (PATH may not be refreshed).
- The user speaks Spanish; answer in Spanish in chat, but the REPO TEXT
  stays English per his explicit round 9 decision.
