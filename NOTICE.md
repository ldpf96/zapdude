# NOTICE - Zapdude

Copyright (c) 2026 ldpf96 (https://github.com/ldpf96)
License: **GNU Affero General Public License v3.0** (see `LICENSE`).

## Origin and credits

- **Design reference.** Zapdude's interface was designed with
  [ZapZap](https://github.com/rafatosta/zapzap) by Rafael Tosta as its
  reference: the account rail, one-click account creation, opening
  straight into the QR screen and the tray flow follow that model.
- **Implementation.** Zapdude is an original implementation written in
  Rust for Tauri 2 (WebView2/WKWebView/WebKitGTK). It shares no source
  with ZapZap, which is a Python/GTK application, and the product has
  since grown into its own shape: a full-screen options drawer, a local
  automation API for people and AI agents (REST + SSE + signed
  webhooks), instant account switching and per-account call pop-outs.
- **Development tooling.** The author implemented Zapdude with AI
  assistance (the OpenCode agent harness driving the GLM-5.3 model by
  Z.ai), reviewing and verifying every change on real hardware before
  release.
- **License rationale.** AGPL-3.0 was chosen so that anyone building on
  Zapdude - or serving a modified version to others over a network -
  shares the corresponding source.

## Trademark and ownership

- Zapdude is an independent project and is **not affiliated with,
  endorsed or sponsored by** WhatsApp or Meta Platforms, Inc.
  **WhatsApp** is a trademark of Meta Platforms, Inc. The app only
  displays your own WhatsApp Web session; your data stays yours.
- **ZapZap** is and remains the project of Rafael Tosta.

## License

Copying, modifying and redistributing Zapdude is free under the terms
of the GNU AGPL v3. Modified works stay AGPL-3.0 or later.
