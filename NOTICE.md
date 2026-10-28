# NOTICE - Zapdude

Copyright (c) 2026 ldpf96 (https://github.com/ldpf96)
License: **GNU Affero General Public License v3.0** (see `LICENSE`).

## Origin and honest attribution

Zapdude began as a personal take on the ZapZap concept. The story is
straightforward and we want it straight on the record too:

1. **Where the idea comes from.** The starting point was
   [ZapZap](https://github.com/rafatosta/zapzap) by Rafael Tosta, a
   WhatsApp Web multi-account desktop client written in Python (GTK).
   ZapZap's source code was used as a **reference for design and
   behavior**: features like the account rail, one click to add a
   WhatsApp account, opening straight into its QR code, the system tray
   behavior and the general interaction flow were deliberately modeled
   after what ZapZap does.

2. **What the code is.** The implementation shipped here is not a copy
   of ZapZap's code. ZapZap is written in Python against GTK; Zapdude is
   written in Rust against Tauri 2 and WebView2, from a different stack,
   file layout and architecture. Text level, the two codebases share
   nothing. Behavior level, the early versions intentionally tried to
   feel like ZapZap, and over time the project grew into something
   substantially different: options rendered in a full screen drawer,
   a local automation API for people and AI agents with SSE and signed
   webhooks, immediate account switching, per account call pop-outs and
   more.

3. **Why AGPL-3.0.** The author picked the GNU AGPL v3 (the license of
   this project) so the code released here carries a strong copyleft:
   anyone who builds on Zapdude or serves a modified version to others
   under the network terms of AGPL 13 must share the corresponding
   source. We are not stating that this project is free of design
   obligations toward anyone: we explicitly credit Rafael Tosta as the
   author of the original ZapZap client whose design inspired this one.
   If you believe any part of this project infringes your rights, open
   an issue and we will fix it.

4. **AI assisted development.** This project was implemented by its
   author with a heavy assist from AI tooling: the **OpenCode** agent
   harness driving the **GLM-5.3** model (by Z.ai), working in what is
   casually known as **vibe coding**: fast, intent driven iterations with
   the agent writing the bulk of the code and the author steering,
   reviewing and verifying everything on real hardware before shipping.
   We think this should be said plainly instead of assuming AI is hidden
   somewhere.

## Trademark and ownership

- **Zapdude** is an independent community project.
- It is **not affiliated with, endorsed or sponsored by** WhatsApp or
  Meta Platforms, Inc. **WhatsApp** is a trademark of Meta Platforms,
  Inc. All chat data belongs to each user; Zapdude only shows your own
  WhatsApp Web.
- **ZapZap** is the project of Rafael Tosta and remains his work.

## License

- Copying, modifying and redistributing Zapdude is free under the terms
  of the GNU AGPL v3. Modified works stay AGPL-3.0 or later.
- Attribution notes: mentioning Zapdude (and ZapZap, by Rafael Tosta)
  when your work derives from them is appreciated and honest, though
  the license text is what legally applies.
