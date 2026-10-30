# Zapdude 1.3.1 — the polish pass

Three fixes straight from the first real-call validation round. **AGPL-3.0.**

## Fixed

- **Missed-call toasts wear the same clean round avatar the incoming call
  toast used.** WhatsApp builds the missed-call Notification with its own
  composite icon (the contact photo re-cropped plus the WhatsApp badge
  stamped on) - it stood out badly next to the branded incoming toast.
  The ring surface now stores its channel avatar keyed by the caller's
  handle, and the missed-call hook swaps the composite for it before the
  toast renders. If a missed call ever arrives without a preceding ring,
  WhatsApp's icon stays (degrade, never crash). Verified live: the
  missed event's icon is the exact CDN URL the ring surface carried.

- **The "Descarga WhatsApp para Windows" promo card is gone from the
  empty state.** WhatsApp's v2 intro card (illustration + the
  extra-features line mentioning calls + the Descargar button) carries
  its own semantic testid, so the hide is precise by construction. The
  functional action tiles below it (Enviar documento / Añadir contacto /
  Pregúntale a Meta AI) stay.

- **No more one-second "create an account" flash at every startup.**
  With accounts configured, the shell boots the welcome webview as a
  pure brand splash and lands straight in the first account. The full
  account picker now belongs to the real no-accounts state only - where
  it is the intended destination - and re-gains its add buttons on the
  delete-last-account path.

## Downloads

- Windows: `Zapdude_1.3.1_x64-setup.exe` (NSIS, per-user install)
- macOS Apple Silicon: `Zapdude_1.3.1_aarch64.dmg`
- macOS Intel: `Zapdude_1.3.1_x64.dmg`

Zapdude is free software under GNU AGPL-3.0 - source on
[github.com/ldpf96/zapdude](https://github.com/ldpf96/zapdude).
Not affiliated with WhatsApp, Messenger or Meta Platforms.
