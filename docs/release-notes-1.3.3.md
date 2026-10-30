# Zapdude 1.3.3 — the conversation pane is sacred

A one-field hotfix for a serious regression. **AGPL-3.0.**

## Fixed

- **Clicking a chat could leave the right side permanently GRAY** (the
  click still marked the chat read, but the conversation never
  rendered). The "hide WhatsApp's download promo card" feature painted
  `display:none` on the card and then collapsed "promo-only" ancestors
  - and the card sits high in the right pane's tree, next to
  containers the conversation view mounts into. During WhatsApp's
  intro→chat transitions a sweep could collapse a shared container,
  and the chat view was born inside it, invisible.
- The hide is now a pure selector rule
  (`[data-testid="intro_panel_v2_title_card"]`, injected once as a
  style tag - the same technique the call styling uses). It never
  climbs, never paints an element, and while the card is unmounted it
  matches nothing at all. Verified live: opening a chat renders the
  header, the message body and the composer every time.

## Downloads

- Windows: `Zapdude_1.3.3_x64-setup.exe` (NSIS, per-user install)
- macOS Apple Silicon: `Zapdude_1.3.3_aarch64.dmg`
- macOS Intel: `Zapdude_1.3.3_x64.dmg`

Zapdude is free software under GNU AGPL-3.0 - source on
[github.com/ldpf96/zapdude](https://github.com/ldpf96/zapdude).
Not affiliated with WhatsApp, Messenger or Meta Platforms.
