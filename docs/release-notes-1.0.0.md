# Zapdude 1.0.0 - first public release

Multi account WhatsApp Web desktop client for Windows, written in Rust
(Tauri 2 + WebView2) by ldpf96. License AGPL-3.0.

## Install

1. Download `Zapdude_1.0.0_x64-setup.exe` (about 2.81 MiB) from this
   release.
2. Run the installer (current user mode, no admin needed).
3. The app opens your first account and its QR right away. On your
   phone: WhatsApp, Linked devices, Link a device, then scan.

## What is inside

- All your WhatsApp accounts in one window with a left rail.
- Instant account switching (idle accounts stay parked outside the
  window at full size, so WhatsApp does not re-render).
- Isolated WebView2 profiles per account (sessions never mix).
- Full screen options: light/dark theme, start with Windows, start
  minimized, account renaming, API settings.
- System tray with "Show <account>", hide everything and quit.
- WhatsApp Web calls (stable Chrome identity and pre-approved
  permissions for microphone, camera and screen sharing).
- Local API (loopback) for people and AI agents: REST + SSE + webhook
  with HMAC signature (`X-Zapdude-Signature`). Disabled by default.

## Notes

- Needs Windows 10/11 with the WebView2 Runtime (bundled with Windows
  11).
- Original project by [ldpf96](https://github.com/ldpf96). The design
  and behavior reference was ZapZap by Rafael Tosta; no ZapZap code
  ships here. Development credits: OpenCode + GLM-5.3 (by Z.ai), vibe
  coding, verified on real hardware. Not affiliated with WhatsApp or
  Meta.
- Source: https://github.com/ldpf96/zapdude (AGPL-3.0).
