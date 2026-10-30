# Zapdude 1.3.2 — the badges

The rail now talks like a phone. **AGPL-3.0.**

## Added

- **Unread count bubbles on the account tiles** (the mobile-style badge).
  Every account page samples its own document title - WhatsApp Web and
  Messenger both publish the unread counter there natively (`(4)
  WhatsApp`) - through a shared, drift-proof watcher. The data lands in
  the host store per account and arrives on the rail with every poll;
  the tile grows a green bubble on its top-right corner (the session
  dot keeps the bottom one). Counts clamp at 99+; zero unread = no
  bubble at all.

- **The toggle.** A new "Unread badges" switch lives in the drawer's
  Preferences: off hides every bubble at CSS level (stale counts can
  never pop one back mid-poll), on brings them back on the next poll
  beat. The preference ships default-on for older configs.

- Also aboard (committed between 1.3.1 and this tag): the drawer's
  About card carries a **Search updates** button - the light tier of
  the auto-updater (quiet once-per-open check announcing newer tags, a
  direct Download link, full status on demand).

## Downloads

- Windows: `Zapdude_1.3.2_x64-setup.exe` (NSIS, per-user install)
- macOS Apple Silicon: `Zapdude_1.3.2_aarch64.dmg`
- macOS Intel: `Zapdude_1.3.2_x64.dmg`

Zapdude is free software under GNU AGPL-3.0 - source on
[github.com/ldpf96/zapdude](https://github.com/ldpf96/zapdude).
Not affiliated with WhatsApp, Messenger or Meta Platforms.
