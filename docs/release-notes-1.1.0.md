# Zapdude 1.1.0 - English everywhere and an honest origin story

Same app, now with all its text in English, the LICENSE confirmation,
and a fully honest origin note in the project docs.

## What changed

- ALL project text is now in English: UI (rail, options drawer,
  welcome), tray menu, installer language, docs and code comments.
- Honest origin story in NOTICE.md and README.md: the design reference
  was ZapZap by Rafael Tosta, early versions tried to feel like ZapZap,
  and the implementation grew into something substantially different.
  There is no ZapZap code text in the project (different language and
  stack), and the original concept is credited explicitly.
- AI development credit on the record: OpenCode agent harness + the
  GLM-5.3 model (by Z.ai), vibe coding style, every patch verified on
  real hardware by the author before shipping.
- Windows installer ships English only.
- Installer and app icon stay consistent: the approved green squircle
  with the black bolt on the title bar, taskbar, tray and installer.

## Installer

- `Zapdude_1.1.0_x64-setup.exe`: NSIS by user, no admin, English UI.
  Uninstalling wipes the app data folder.

## Current platform status

- Windows: supported and shipped here (installer + source).
- Linux: attempting a CI build (deb/AppImage). If it takes more than a
  reasonable effort it will be dropped per the author's instruction.
- macOS: NOT supported by this architecture (the single-window shell
  requires Tauri's multi-webview, which is Windows/Linux only), so
  there is no macOS build on purpose, and that is documented here.

## Notes

- Original project by [ldpf96](https://github.com/ldpf96). The design
  reference was ZapZap by Rafael Tosta; created with AI assistance
  (OpenCode + GLM-5.3, vibe coding). Not affiliated with WhatsApp or
  Meta. License AGPL-3.0.
- Source: https://github.com/ldpf96/zapdude
