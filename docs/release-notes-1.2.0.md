# Zapdude 1.2.0

The app-wide look arrives: light and dark themes now cover the entire
window - chat area included - and macOS gets its first installer.

## Highlights

- **App-wide theming (Windows).** The theme selected in the options
  drawer now drives the whole window: each account's WebView2 profile
  pins its color scheme (`prefers-color-scheme`), so WhatsApp Web
  renders in the matching palette, live, without reloading the page or
  touching the session. Native caption bars follow the theme too.
- **macOS support (first version).** Signed-off `.dmg` installers for
  Apple Silicon and Intel, built by CI, nothing paid, no Apple
  Developer Program.

## macOS installation

Choose the build for your Mac:

- `aarch64` = Apple Silicon (M1/M2/M3/M4 or newer).
- `x64` = Intel Mac.

Open the `.dmg`, drag **Zapdude** to Applications, then open it. Zapdude
is distributed directly (not through the App Store) and is not notarized
with Apple's paid signing, so macOS adds one confirm step the first
time it is opened:

- If macOS shows "Zapdude can't be opened", run this once in Terminal:
  `xattr -dr com.apple.quarantine /Applications/Zapdude.app`
- Or open System Settings → Privacy & Security and press "Open Anyway".

That is a one-time step; from then on the app opens like any other.

macOS 14 (Sonoma) or newer is required.

## Windows installation

- `Zapdude_1.2.0_x64-setup.exe`: per-user NSIS installer, no admin
  needed, English interface.

## Platform notes

- macOS: the embedded chat follows the Mac appearance setting (the
  in-app theme switch covers it from a future release); call pop-out
  windows are a Windows feature for now.
- Windows: everything as in 1.1.0, plus app-wide theming.
- Linux: planned next.

## Credits

- Original project by [ldpf96](https://github.com/ldpf96). Design
  reference: ZapZap by Rafael Tosta. Developed with AI assistance
  (OpenCode + GLM-5.3). Not affiliated with WhatsApp or Meta.
  License AGPL-3.0.
- Source: https://github.com/ldpf96/zapdude
