# Release notes - Zapdude 1.3.0

**Zapdude 1.3.0** - from a WhatsApp-only desktop client to the first
multi-service chat hub, backed by the same local automation API.

## Highlights

### Facebook chat is now a first-class citizen
Add a Facebook account from the rail's "+" strip and sign in **inside the
app** (email + password + 2FA, exactly like on the web). The embedded view
stays anchored on your chat, never the full Facebook feed. Sessions are
perceived identically to WhatsApp's: one profile per account, logged in
until Facebook says otherwise.

### The local API now speaks accounts, not just WhatsApp accounts
`GET /api/accounts` and `/api/status` carry a `service` field for every
account, and every event is tagged with the account AND its service.
Receive-side operations (`status`, `chats`, events, webhooks) work across
services; WhatsApp-only operations (`message`, `open-chat`, calls) answer
a clean `unsupported_for_service` on other service accounts, so agents
never hit mystery errors.

### Notifications, the way you asked
- Uniform branded toasts: contact name (bold) + message + a small round
  contact avatar at the left. The redundant app title and the raw
  `account-1` tag are gone.
- Clicking a notification opens the chat that produced it.
- Incoming calls surface as the same branded toast (caller + avatar),
  useful even for accounts parked in the background; the visible call
  prompt itself is cosmetically restyled (rounded, softer shadows,
  brand text) while every button keeps working. The caller's own
  "Calling..." surface stays silent (one call, one toast, the callee's).
- Missed calls keep the caller's PHOTO and human name too: the big
  inline avatar payloads decode now (the old length gate silently
  dropped them), and the handle-to-name pair learned at ring time
  replaces WhatsApp's bare-handle titles.
- Every notification is THE SAME SIZE at birth: icon slot always filled
  (contact avatar or the app icon), one-line bold title, exactly two
  body lines - messages, calls and missed calls read as one family.
- The square notification is gone: WhatsApp's own service worker
  (sw.js) raised un-stylable OS notifications from its realm; Zapdude
  intercepts the registration and sweeps it away, so the branded toast
  is the only surface.
- State you can read at a glance on the rail: green = connected,
  yellow = waiting for QR / sign in, red = a page problem, grey = loading
  - without the color-rotation the older releases showed.

### The add-account strip
Click the "+" and a brand strip extends beside the tile with the real
WhatsApp and Messenger logos (icons from Simple Icons, CC0/public domain,
attributed in NOTICE.md).

### Polish and safety
- The "download our desktop app" banner hider now covers the short
  Spanish texts too.
- New `status` differentiation: `needs_login` (Facebook login screen)
  vs `detecting` vs `logged_in`, plus a red-dot `page_error` state in
  the rail for a failing page.
- Stable backups: every verified build can be snapshotted (binary +
  full source + manifest) with `tools/backup-stable.ps1`.

## Known notes
- The Facebook send-side (reading thread contents deeper, sending
  messages, calls) is deliberately NOT in this release: receive-side
  first, per the project doctrine that the API degrades clearly instead
  of guessing.
- Call-surface styling applies to the WhatsApp-in-page call prompt only;
  it self-degrades if the markup moves, and the prompt stays functional
  in every case.

**Next on the roadmap**: a CHECK FOR UPDATES button inside the app that
talks to the GitHub releases endpoint (and, with signing keys wired,
full one-click self-updates). Cookies for now: notify-on-new-release.

**Mac users**: the 1.3.0 macOS builds follow the 1.2.1 Ventura caveat
(macOS 13+ minimum tested as experimental; 14+ is the proven floor).
