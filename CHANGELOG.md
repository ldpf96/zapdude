# Changelog

Todos los cambios notables de este proyecto se documentan aquí.
El formato sigue de cerca Keep a Changelog; versionado SemVer.

## [1.0.0] - 2026-10-28

### Added
- Reescritura completa en Rust del cliente ZapZap (Python/GTK) de Rafael
  Tosta sobre Tauri 2 + WebView2.
- **Shell v3 de ventana única** (feature `unstable`): rail de cuentas (76 px),
  un webview de WhatsApp Web por cuenta con perfil WebView2 aislado
  (`webview/<id>`), backdrop de bienvenida y panel de opciones a pantalla
  completa en la misma ventana (logo del rail = toggle).
- **Multi-cuenta vital**: "+" agrega cuenta sin formularios y abre su QR al
  instante; cambio de cuenta instantáneo por parking position-only
  (parqueo a tamaño completo fuera de la ventana, sin re-render: <350 ms);
  carga diferida — cada cuenta carga WhatsApp Web la primera vez que se abre.
- **Rail premium**: logo Zapdude, avatares con el rayito (sin letras/números),
  anillo verde en la activa, dot de estado (conectada/QR/cargando),
  divisorias curvas, X roja visible al armar el borrado con clic derecho,
  actualización incremental del DOM (sin parpadeo). Sin menú contextual del
  navegador en la UI.
- **Bandeja del sistema**: "Mostrar <cuenta>" por cuenta, "Ocultar
  ventanas", "Salir de Zapdude". Cerrar la ventana oculta a la bandeja.
  Instancia única por mutex.
- **Identidad Chrome estable** (UA 156 + `client_hints_fix.js`) y permisos
  preaprobados de micrófono/cámara/pantalla/notificaciones/portapapeles
  para llamadas de WhatsApp Web.
- **API local para humanos y agentes**: REST (`healthz`, `config`,
  `accounts` CRUD + open/hide, `status`, `open-chat`, `message`, `chats`,
  `events`, `webhook`) + SSE (`/events/stream`), loopback por defecto,
  `X-API-Key`/Bearer, 60 rpm, cuerpo ≤ 1 MB, eventos etiquetados por cuenta
  y webhook HMAC-SHA256. La UI (rail/opciones/bienvenida) la sirve el propio
  servidor (mismo origen, sin IPC).
- **Limpieza automática de promos de WhatsApp**: banner "Download WhatsApp
  for Windows" del landing y tarjeta de descarga del panel vacío, con
  zonas protegidas (jamás toca el QR, la lista de chats ni el compositor).
- Puente de página residente (`api_page_bridge.js`) con transporte primario
  `eval_with_callback` y beacon de hash URL `#__zapdude=` como reserva,
  hilo poller de 30 ms por cuenta; scripts diferidos con marca de
  idempotencia por script.
- Borrar la última cuenta muestra la pantalla de bienvenida con el botón
  "Agregar cuenta" (no auto-crea cuentas vacías); el arranque sí abre la
  primera cuenta (o crea "Cuenta 1") como el ZapZap original.
- Configuración persistente en `%APPDATA%\app.zapzap.desktop\config.json`
  (camelCase, tolerante a BOM) con `api` y `accounts` sanitizados.
- Instalador NSIS (`tauri build --bundles nsis`, `installMode: currentUser`,
  selector de idioma, hook que borra los datos al desinstalar).
- Licencia AGPL-3.0 y `NOTICE.md` con crédito a Rafael Tosta.

### Fixed
- Página gris de WebView2 (2 causas): webviews nacidos ocultos o enanos no
  arrancan su pipeline de composición (nacen a tamaño completo y se
  parquean por posición), y el script viejo de promos ocultaba contenedores
  enteros (reescrito smallest-node-only con colapso supervisado).
- Inyección de scripts: solo el mínimo en document-start; el bridge y el
  hider van diferidos vía poller (un eval grande en document-start rompe el
  render de WebView2).
- `ExecuteScript` de WebView2 llega doble-codificado cuando el valor es
  String: en `evaluate` se desenrolla una vez más (los datos ya no caen a
  defaults "vacíos").
- Anidadas en axum 0.8: rutas planas `/api/...` (`.nest("/")` no existe y
  paniqueaba en runtime).
- `session_state` en 3 estados honestos: `detecting` / `scan_qr` /
  `logged_in`, con caché de 2.5 s para que el rail responda al instante.
- El servidor API arranca ANTES que los webviews (adiós error de Edge "no
  se puede alcanzar esta página") y el bind reintentа 40×250 ms (el
  restart tras cambiar el puerto chocaba con el error 10048).
- Botones promo cuyo `textContent` arrastra el título del icono SVG
  ("Downloadic-download"): `cleanText()` elimina tokens `ic-*` antes de los
  patrones exactos; el instalador diferido usa una marca por script para
  que un fallo del bridge no silencie al limpiador de banners.
- **Icono de la app** (Round 7): el `icon.ico` incrustado en el exe seguía
  siendo la versión vieja (teal oscuro con rayo blanco); reconstruido
  byte a byte desde los PNG aprobados (verde con rayo negro) en 16/24/
  32/48/64/128/256; `icon.png` (512) y las mosaicos UWP re-renderizados
  con el mismo diseño (scripts/regen-icons.ps1). Verificado leyendo el
  HICON real de la ventana en ejecución.
- **Panel de opciones lento al cerrar** (Round 7): `toggle_drawer` no
  realineaba en el cierre y esperaba al watchdog de geometría de 2 s;
  ahora `relayout` corre en el mismo toggle (medido: 1 ms en estacionar).
- **Renombrar cuentas borraba lo escrito** (Round 7): la lista del drawer
  se reconstruía desde cero cada 5 s (reset del valor y robo del foco a
  mitad de la escritura). Ahora el refresco es incremental por id, y jamás
  toca un input enfocado o editado; fusiona cambios del servidor solo en
  filas intactas.
