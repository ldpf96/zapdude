# Zapdude 1.1.0 - Full English release

The whole product speaks one language now: interface, installer, tray
menu and documentation are all in English, and the project notices were
consolidated.

## What changed

- All project text is English: UI (rail, options drawer, welcome),
  tray menu, installer language, docs and code comments.
- Project origin and credits consolidated in NOTICE.md and README.md:
  the design reference is ZapZap by Rafael Tosta, the implementation is
  original Rust code, and development tooling (OpenCode agent harness +
  GLM-5.3 model by Z.ai, with the author verifying every patch on real
  hardware) is documented alongside.
- Windows installer ships English only.
- Installer and app icon stay consistent: the green squircle with the
  black bolt appears identically on the title bar, taskbar, tray and
  installer.

## Installer

- `Zapdude_1.1.0_x64-setup.exe`: NSIS, per-user, no admin, English UI.
  Uninstalling cleans the app data folder.

## Platform status at this release

- Windows: supported and shipped here (installer + source).
- macOS and Linux: source-supported architecture, with packaged builds
  landing in the following releases (macOS from 1.2.0).

## Credits

- Original project by [ldpf96](https://github.com/ldpf96). Design
  reference: ZapZap by Rafael Tosta. Developed with AI assistance
  (OpenCode + GLM-5.3). Not affiliated with WhatsApp or Meta.
  License AGPL-3.0.
- Source: https://github.com/ldpf96/zapdude
