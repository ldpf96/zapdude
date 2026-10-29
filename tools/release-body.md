Download page: https://ldpf96.github.io/zapdude-site/

**Windows** - `x64-setup.exe`, per-user NSIS installer, no admin needed.
**macOS** - `.dmg` for Apple Silicon (aarch64) and Intel (x64). Minimum system: **macOS 13 (Ventura)**; the shell is verified end to end on macOS 14+, so on Ventura treat it as an early build - if anything looks off, updating macOS restores the proven path. The first open asks for one confirmation because builds ship without paid Apple signing.

**New in 1.2.1**
- One uniform notification surface: native Windows toasts branded Zapdude, with the contact avatar and click-to-open. Real message events land in the API (SSE, polling, webhook) for every account.
- Every enabled account opens at startup, parked and alive: notifications work for all accounts from the first second.
- Calls: `POST /api/call` and `POST /api/call/end` drive the WhatsApp call flow; popup windows report `popup_opened` / `popup_closed` and the page reconciles a call popup that was closed without ending the call (no more stuck "in call" state).
- Fixed: the media download option no longer disappears; chat list scraping follows the current WhatsApp markup with unread counts; repeated launches bring the window back instead of dying silently (autostart stays tray-minimized).
- API hardening: reliable chat opening through the search drawer, page bridges replaced on webview recreation, and the live debug harness (`ZAPDUDE_DEBUG_EVAL=1`).

**Source** - https://github.com/ldpf96/zapdude (AGPL-3.0; not affiliated with WhatsApp or Meta).
