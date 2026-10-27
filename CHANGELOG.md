# Changelog

Todos los cambios notables de este proyecto se documentan aquí.
El formato sigue de cerca Keep a Changelog; versionado SemVer.

## [1.0.0] - 2026-10-27

### Added
- Reescritura completa en Rust del cliente ZapZap (Python/GTK) de Rafael
  Tosta sobre Tauri 2 + WebView2.
- Multi-cuenta real: una ventana por cuenta (`account-<id>`) con perfil
  WebView2 aislado (`data/webview/<id>`), solo la primera visible; cerrar
  ventana oculta; salida desde la bandeja.
- Bandeja del sistema: "Show <cuenta>" por cuenta, "Hide all windows",
  "Exit ZapZap".
- Identidad Chrome estable (UA 156 + `client_hints_fix.js`) y permisos
  preaprobados de micrófono/cámara/pantalla/notificaciones/portapapeles
  para llamadas de WhatsApp Web.
- API local para humanos y agentes: REST (`healthz`, `status`, `accounts`,
  `open-chat`, `message`, `chats`, `events`, `webhook`) + SSE
  (`/events/stream`), loopback por defecto, `X-API-Key`/Bearer, 60 rpm,
  cuerpo ≤ 1 MB, eventos etiquetados por cuenta y webhook HMAC-SHA256.
- Puente de página residente (`api_page_bridge.js`) con transporte primario
  `eval_with_callback` y beacon de hash URL `#__zapdude=` como reserva,
  hilo poller de 30 ms por cuenta.
- Configuración persistente en `%APPDATA%\app.zapzap.desktop\config.json`
  (camelCase, tolerante a BOM) con `api` y `accounts` sanitizados.
- Instalador NSIS (`tauri build --bundles nsis`, `installMode: currentUser`).
- Licencia AGPL-3.0 y `NOTICE.md` con crédito a Rafael Tosta.

### Fixed
- `ExecuteScript` de WebView2 llega doble-codificado cuando el valor es
  String: en `evaluate` se desenrolla una vez más (los datos ya no caen a
  defaults "vacíos").
- Anidadas en axum 0.8: rutas planas `/api/...` (`.nest("/")` no existe y
  paniqueaba en runtime).
- `session_state` en 3 estados honestos: `detecting` / `scan_qr` /
  `logged_in`.
