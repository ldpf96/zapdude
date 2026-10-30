# Session status - Zapdude (Rust)

> Continuity document for AI agents and future sessions. Last updated:
> 2026-10-31 morning - post-1.3.0 user-report fixes (see the STATE OF
> THE WORLD block below; the 2026-10-30 closing block at the end holds
> the call/notifications history).

> **STATE OF THE WORLD** (2026-10-31 evening): repo main -> origin
> ldpf96/zapdude; **v1.3.2 LIVE + installed** (badges + updater button).
> The EVENING shipped a serious hotfix the user reported IN PRODUCTION:
> **clicking a chat left the right pane GRAY** (read-marked but never
> rendered). Root cause: the empty-state promo hide PAINTED display:none
> on the card and climbed collapsible ancestors - the card sits HIGH in
> the right pane's tree; during WhatsApp's intro->chat transitions a
> sweep caught a transient with only hidden children and collapsed a
> SHARED container the conversation mounts into. FIX (1.3.3, verified
> live: open-chat -> #main rendered + composer visible): the hide is a
> SELECTOR RULE now (style tag, intro_panel_v2_title_card - same
> technique as call styling), never element painting, never ancestor
> climbing. **LESSON for future session: element-painting hides +
> ancestor climbs are FORBIDDEN near shared containers - selector rules
> only.** v1.3.3 cut: commit e418bea + tag -> CI 41119252254; publish
> staged (publish-133 flow).

> Older STATE block (2026-10-30 mid-session, superseded): v1.2.1 is the
> last PUBLISHED release; the user's PC runs the OLD **v1.2.0** installer
> edition (their "start-menu click does nothing / opens only in the tray"
> symptoms: both fixed since 1.2.1 - second-launch `/api/ui/show` ping +
> manual launches always show the window - and cured entirely by
> installing the 1.3.0). The working tree is **v1.3.0** with the call
> part-2 afternoon folded in (uniform toast law, sw-guard square-
> notification fix, missed-call photo+name, startCall label drift,
> invisible dev launcher); the NSIS 1.3.0 re-cut is running with ALL of
> it (the 09:37 cut is stale, do not install from it). PUBLISH flow:
> local folder is NOT a git repo yet - init + remote + commit + tag
> first; gh is authed as ldpf96 (scope repo).

> **SESSION CLOSE PROTOCOL (2026-10-30)**: the user comps each session;
> keep this block as the entry ramp. Backups: several
> `backups/stable-dev-<stamp>` snapshots exist (tools/backup-stable.ps1;
> the newest 12 are kept). The dev launcher is `tools/run-dev.cmd`
> (ZAPDUDE_DEBUG_EVAL=1, console to zapdude-console.log). The INSTALLED
> app on the user's PC is still the OLD v1.2.0 unless they run the new
> NSIS installer - any double-click of the OLD shortcut silently
> resurrects the old binary (it rewrote config.json to its schema once,
> wiping `service` fields), so v1.3.0 install replaces it.

## Closing block - what v1.3.0 carries (all verified as noted)
## What changed on 2026-10-30 (working epoch, knowledge for future sessions)

- **Toast anatomy rebuilt (user feedback: "la foto sale abajo y gigante",
  "dice account-1 en lugar del nombre").** Root cause: `notifier.rs` used
  `Toast::image()` (tauri-winrt-notification) which ships the image with NO
  placement attribute - Windows renders that as a full-width banner at the
  BOTTOM of the toast. Also the layout was title "Zapdude" + `[account-1]`
  id glued at the tail of the message. New anatomy:
  `.icon(path, IconCrop::Circular, ...)` -> appLogoOverride round avatar at
  the LEFT; contact name as the bold first line (the redundant "Zapdude"
  title was dropped: the AUMID footer already attributes Zapdude); message
  as line 2; account LABEL (renamed via the drawer, resolved through
  `SHARED.settings.current_accounts()` by id) as line 3 only in
  multi-account shells (<= 1 enabled account = no account line, no
  click-through, the footer covers it).
- **Toast click-through**: activation now opens the account that produced
  the event (`open_account_webview`, same choreography as tray
  "Show <account>": window first, webview on a worker thread; re-checks the
  account still exists at click time so a deleted account is not
  resurrected from the snapshot).
- **Verified live** 2026-10-30: 14/14 unit tests green; debug build rebuilt
  and the user's dev instance restarted with the fix; synthetic
  `POST /api/_debug/emit` (kind "incoming", data.kind "notification",
  account-1, data-URL icon) produced the toast on the user's screen and
  decoded the icon to
  `toast-icons/avatar-test_zapdude_toast.png` (533 B = the source icon).
  Live-tested labels: config has account-1 -> "+51987931332" and
  account-2 -> "Account 2" (2 enabled accounts = the line shows).
- **CALL notifications go UNIFORM (user request: "hazla como las de
  mensajes, que se ven muy bonitas").** The incoming-call prompt is
  WhatsApp's own in-page surface, so the branded look now comes from the
  unified pipeline instead: the call anchor script, when a call surface
  mounts, emits the SAME `notification` payload the message notifications
  use (title = caller identity best-effort: `@handle` match or short
  line, body = "Incoming (voice|video) call", and the caller avatar
  decoded from the surface `<img>` blob: URL -> fetch -> FileReader ->
  data URL, which the notifier's icon cache consumes exactly like message
  icons). Result: the SAME branded native toast (round avatar, bold
  name) for incoming calls, usable even for parked/hidden accounts that
  would otherwise ring invisibly. Guards: WeakSet identity dedupe (the
  ongoing-call panel mutates constantly - no toast spam), active
  (answered) surfaces are excluded from toasting, all best-effort. The
  prompt itself stays functional; once the surface outerHTML capture
  lands (next real call), the styling pass gives it the brand look
  without touching the buttons. Build verified 16/16 + backup
  `stable-dev-20261030-092102`.
- **STABLE BACKUPS discipline (the user demanded it after the session's
  rail outage).** NO git repo exists in this working folder, so the anchor
  is: (a) `tools/backup-stable.ps1` - run AFTER a verified build; makes
  `..\backups\stable-dev-<stamp>\` with zapdude.exe + .pdb + source.zip
  (tar excludes target/ + logs) + a MANIFEST.txt of what passed, prunes to
  the newest 12; (b) GitHub release artifacts (CI-built) for real
  releases. Snapshots so far: `stable-dev-20261030-083233` (rail recovery
  + facebook phase 1), `stable-dev-20261030-085537` (+ pinned strip,
  click-open, promo v2 patterns, call-surface HTML capture). HONEST
  NOTE: earlier snapshots of pre-today states were NOT taken on disk -
  the exe changes in place with every build; recoverable prior-stable
  source lives in the v1.2.1 RELEASE (GitHub) + the backups from now on.
- **Session refinement batch (user screenshot feedback, 2026-10-30 late
  morning).** Four items:
  1. Desktop-banner promo leaked on account-2: the hider's Spanish
     patterns covered "descarga la aplicaci[on]" but the umbrella flag
     text was likely the SHORT "descarga la app" - patterns extended
     (descarga/obten/instala + "la app", "download our app", "get our
     app", "WhatsApp Desktop"/"para escritorio", "con whatsapp en tu
     computad[ora]"). The smallest-match rule + collapse climb + protected
     zones stay the safety rails. The bottom banner's X button is an
     svg-only child (promo-ish) so the climb can collapse the bar.
  2. Call notifications look "cuadradas y toscas": the incoming-call
     prompt (ACCEPT/DECLINE) is WHATSAPP's OWN in-page surface (it never
     flows through the Notification API, so the branded native toast
     cannot carry it; missed calls DO). No styling by guess work - the
     call_anchor script now captures the surface outerHTML (6KB cap)
     through the bridge, so the NEXT real call provides the exact classes
     and testids for a brand-styling pass (phase 2 pending that data).
  3. The strip menu sat misaligned vs the + (drawered screenshot): the +
     tile was INSIDE the scroll area, so with few accounts it drifted up
     right after the avatars while the strip anchored to the client
     bottom. FIXED: .add-wrap moved OUT of .rail-scroll as a body-level
     sibling - the + is PINNED to the rail bottom edge always; the strip
     slot geometry (RAIL_WIDTH+6, H-74, 178x74) now hugs it exactly.
  4. The strip now opens by CLICK (+ click -> /toggle), per the user's
     preference; the hover opener was removed entirely. Soft-close grace
     on leaving the + grew to 1200ms (touch pings cancel it - the
     4px gap crossing). The strip ALSO got a self-kill fix: close-on-
     leave now ARMS only after a real mouseenter (the birth-shrink
     synthesized leaves raced the fixed 700ms guard and some mounts
     self-closed the menu before it was used). toggle_drawer now
     dismisses the strip so the options page never has it floating.
- **RAIL SCRIPT CRASH root cause (the user's "accounts disappeared + hover
  gone" report, 2026-10-30).** During the hover-menu wiring, the rail's
  `addWrap` got USED without a declaration AND a duplicate
  `const hoverTimer` sat in the same scope - one ReferenceError under
  `use strict` killed the ENTIRE rail page script at boot: no poll loop,
  no account avatars, no dots, no hover bindings, drawer still worked
  (host-side). LESSON recorded as a hard rule: whenever editing the shell
  pages, `new Function`-style check or at minimum a string sanity pass on
  the served page (index of every referenced identifier) BEFORE shipping;
  the crash shipped because the rail pages have no automated test today.
  Fixed: addWrap declared, duplicate const removed. Verified live: both
  WhatsApp accounts render, dots stable green.
- **Toast click-through now opens THE CHAT, not just the account (user
  report #3: "te lleva a la cuenta, el chat desaparece, gris").** That
  grey pane was WhatsApp's own "no chat selected" placeholder: the old
  click-through switched accounts but never fired the page-side click.
  NEW: after `open_account_webview`, the host fires the notification's
  OWN click handlers on that account page
  (`window.__zapdudeFireNotificationClick(tag)` - the hook stores every
  instance's onclick; empty tag fires empty-tag instances). The call is
  an async retry loop (6 x 300ms) through the page bridge, tolerating
  bridge mounting beats. Hook presence verified live via debug eval:
  `typeof window.__zapdudeFireNotificationClick` -> "function" on
  account-1. A real user toast click is the remaining live proof.
- **Service picker FINAL shape: in-rail HOVER opening a horizontal strip
  (2026-10-30 late).** The user loved the hover menu but wanted it
  extending to the RIGHT ("lo hiciste para arriba en lugar de hacia la
  derecha"). An on-screen CLIP MARKER test proved the rail webview
  CLIPS at its 76px viewport, so an in-rail horizontal menu is
  impossible. FINAL ARCHITECTURE (all verified live):
  - rail.html keeps only the HOVER trigger: + mouseenter (130ms) ->
    POST /api/ui/picker/show; wrapper mouseleave -> POST
    /api/ui/picker/soft-close; + click -> /toggle; document outside
    click + Esc -> /hide.
  - window.rs: `SERVICE_PICKER_WINDOW` child webview ("service-picker").
    show = DESTROY any existing + add_child born at FULL content size
    (paint law) + relayout immediately shrinks it to the strip slot
    (RAIL_WIDTH+6, content.height-78, 178x78) - the born-full-then-
    shrink path PAINTS correctly (captured), the born-tiny path does
    not. hide = close webview (drawer-style recreate per open keeps
    z-order on top of accounts).
  - ui/picker_strip.html: transparent body + rounded dark card with the
    two brand items horizontal. ALPHAWORKS on child webviews (chat
    visible outside the card) although the top-level alpha sheet failed
    earlier. Self-dismissal: mouseleave (guarded against the synthetic
    leave caused by the birth-to-slot shrink - ignore first 700ms), Esc,
    pick. Cursor presence pings /api/ui/picker/touch so the rail's
    soft-close (420ms grace) does not kill the menu while the cursor
    crosses the 4px gap + -> strip.
  - Endpoints: /api/ui/picker/show|hide|toggle|touch|soft-close (the old
    /api/ui/service-picker* names stay gone).
- **Rail dot semantics fixed (rotating-colors bug).** Root cause:
  SESSION_STATE_TTL (2.5s) EXACTLY matched the rail poll cadence, so the
  cached state expired between every poll and list_accounts answered
  "detecting" while the refresh ran - the dot cycled green/grey forever
  (the user's exact report). Fixed with stale-while-revalidate:
  session_state_entry() serves the stale honest value up to a 30s hard
  cap while the refresh runs in the background; the rail keeps the last
  color until the truth changes. New honest vocabulary the user
  specified: green = connected (logged_in), yellow = needs the human
  (scan_qr OR needs_login), red = real page problem (NEW state
  `page_error`, set by spawn_session_state_refresh when a page evaluation
  fails for any reason other than "bridge not installed yet" - that one
  stays detecting = grey). Rail maps page_error to a red dot + a
  "connection problem" tooltip.
- **Real service logos + ATTRIBUTIONS (user request).** Rail account
  tiles show the REAL service glyphs with brand colors: WhatsApp green
  (#25d366) and Messenger blue (#00b2ff) - both paths from simple-icons
  v13 (CC0/public domain), the same source Franz/Ferdium-class apps use;
  nominative identification of the embedded service. The Zapdude bolt
  remains ONLY on the brand logo top-of-rail. The welcome buttons and
  the hover-menu items use the same two logos. **NOTICE.md now carries
  the full attributions** (simple-icons CC0, embedded web services'
  ownership, trademark not-affiliation wording, runtime dependencies
  pointer) per the user's request to attribute everything borrowed.
- **Messenger QR login = NOT POSSIBLE (answered to the user).** Facebook
  has no cross-device QR pairing for the web chat (that is a WhatsApp
  linked-device exclusive). The equivalent smoothness here is the
  per-account persistent WebView2 profile: sign in once (email + password
  + 2FA) and the session survives restarts like a normal browser;
  session_state needs_login only appears when the profile is logged out.
- **Multi-platform chat hub: FACEBOOK PHASE 1 SHIPPED (2026-10-30).** The
  user's doctrine: NO third-party APIs - Zapdude embeds the REAL web chat
  and offers only its own local API on top ("nuestra app interactúa
  mediante la página web"; the API must NEVER break). Implemented:
  - `AccountConfig.service` ("whatsapp" legacy default, "facebook";
    whitelist + WhatsApp fallback so legacy/typo configs can never wedge).
  - `window.rs`: per-service URL (whatsapp -> web.whatsapp.com, facebook
    -> messenger.com, the standalone chat surface so the embedded view
    stays anchored to the chat, never the full Facebook feed. Login
    happens INSIDE the view: email + password + 2FA exactly like the user
    does on Facebook). Deferred scripts are per service: WhatsApp keeps
    its historic set UNTOUCHED (bridge + promos + call anchor +
    notification hook); Facebook gets ONLY the shared notification hook +
    the new adaptive `scripts/facebook_bridge.js`.
  - `facebook_bridge.js` contract: SAME `window.__zapdudeApiV1` name (the
    host's `bridge_expression` guards stay service-agnostic); every op
    try/catches into structured results, selector strategies are
    candidate LISTS (original + fallbacks), honest `degraded` fields
    instead of fake reads. Ops: pageInfo (incl. `login_form`),
    composerInfo, chatList (NOTE: op name must MATCH the WhatsApp bridge's
    - the first cut used `chats` and the host read "bridge not present";
    renamed and verified), sendCurrent (generic composer write, host
    still gates sends), titleBadge + a 2 s document.title unread watcher
    that ships `title_badge` events ONLY on change (observability, NOT
    toasts).
  - host (`server.rs`): `POST /api/accounts` optional body
    `{service:"facebook"}` (legacy no-body callers unchanged); `service`
    returned by /api/accounts AND /api/status; facebook session states:
    logged_in / needs_login (login form detected) / detecting;
    receive-side endpoints (status, chats, events) are service-agnostic;
    WhatsApp-only ops (message, open-chat, call, call/end) answer HTTP
    400 `unsupported_for_service` for other services - explicit "not
    supported yet" beats silent misfires. Events and chat_opened payloads
    carry additive `service` (old consumers keep working).
  - UI: rail "+" no longer silently creates a WhatsApp account - a mini
    picker WINDOW (`ui/service_picker.html` at /picker, new endpoints
    /api/ui/service-picker + /hide, always-on-top borderless ~250x178,
    anchored above the rail's + tile, hides on blur/Esc/main-window
    focus) asks WhatsApp OR Facebook. The welcome screen grew the same
    two brand buttons. Rail avatars show the service glyph (bolt /
    Facebook f) and the tooltip names the service; accounts list rows in
    the drawer show a service chip. `needs_login` maps to the amber dot
    ("waiting for sign in").
  - **Verified live 2026-10-30**: 16/16 tests; dev build running; the
    picker toggled through the API; a facebook account was created
    (account-3) and the REAL messenger page loaded - status reports
    service=facebook, title=Messenger, login_form_visible=true ->
    session_state needs_login; /api/chats on it answers
    ok:true/degraded:true (no threads while logged out - honest); /api/
    message on account-3 answered the structured unsupported_for_service
    error; both legacy WhatsApp accounts still logged_in (nothing
    regressed); a synthetic notification showed the cleaned toast
    WITHOUT the account line.
- **Toast anatomy closure (user feedback: the account line read as if
  the number belonged to the CONTACT)**: the account label is fully OUT
  of the toast text now (contact + message + round avatar only);
  WinRT's attribution row cannot be rebranded per toast (AUMID display
  name), so the account story lives in the click-through: activation
  opens the exact account that produced it (multi-account shells).
- **Pending user actions**: sign into Facebook inside the embedded
  messenger view (email + password + 2FA) to exercise the live scan:
  thread-row selectors, unread heuristics and the notification capture
  need one real login + real incoming messages before SEND for Facebook
  graduates from gated (`unsupported_for_service`) to supported. Verify
  the toast click-through opens the right account when os notifications
  fire for account-2.


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
- **Secret hygiene (2026-10-29)**: the day's helper scripts initially
  hard-coded the loopback API key and got pushed (tools/debug-eval.ps1,
  tools/sse-listen.cmd). Scrubbed the same session: both helpers now
  read the key at runtime from
  `%APPDATA%\app.zapzap.desktop\config.json`, and the API key was
  ROTATED afterwards - the key inside the pushed git history is DEAD
  (no value left for anyone). The key is only meaningful to local
  processes anyway (the API binds 127.0.0.1 unless expose_network is
  switched on). Keep the runtime-read pattern for any future helper.
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
- The v1.2.1 tag push DID fire the release workflow (the tag-delete +
  re-tag cycle also works cleanly).

## Pending (the NEXT session opens on these)

1. DONE 2026-10-31 evening: **v1.3.3 gray-pane hotfix cut** - commit
   e418bea + tag v1.3.3, CI 41119252254 building; publish staged. THE
   LESSON lives in the STATE OF THE WORLD block: no element-painting
   hides / ancestor climbs near shared containers - selector rules
   only. (v1.3.2 treasures live: unread badges + drawer toggle
   verified both directions.)
2. AUTO-UPDATER: light tier SHIPPED (drawer button). Remaining after
   1.3.2: the DEEP tier - tauri-plugin-updater with release signing
   keys for true one-click self-updates.
3. DONE 2026-10-31: real-call validation (missed-call photo+name,
   uniform sizes, ring sound, no square toast) + the 1.3.1 fixes that
   its feedback produced.
4. Facebook phase 2: only meaningful once a Facebook account exists -
   re-add via the "+" strip when wanted; phase-1 contract stays
   untouched.
5. Backlog (carried): Linux ubuntu matrix row in release.yml, macOS
   toast path (NSUserNotification), own-number endpoint.

## Old pending (from the 1.2.1 era)


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
