# ESTADO DE LA SESIÓN — ZapZap (Rust)

> Documento de continuidad para agentes/compaction. Última actualización: 2026-10-28.
> El usuario reporta que "tiene errores" y enviará una captura — PENDIENTE diagnosticar con esa captura.

## Qué es este proyecto

Reescritura completa en **Rust** del cliente **ZapZap** (original de **Rafael Tosta**,
Python/GTK, https://github.com/rafatosta/zapzap, GPL-3.0). Este fork se distribuye
como **AGPL-3.0** (obra derivada, §13) con crédito obligatorio a Rafael Tosta
(see `NOTICE.md`).

Requisitos del usuario (en orden de importancia):
1. **Multi-cuenta vital**: todas las cuentas de WhatsApp en UNA app, notificaciones
   unificadas. Experiencia EXACTAMENTE como ZapZap original: rail lateral con
   avatares, click en "+" agrega cuenta SIN FORMULARIOS → QR → escanear → entrar.
2. **Interfaz humana** profesional (estilo ZapZap/Apple, dark teal + verde WhatsApp).
3. **API local** en loopback para humanos y **agentes de IA** (REST + SSE + webhook HMAC).
4. **Llamadas/vídeo/pantalla** funcionando (identidad Chrome 156 en JS).
5. **Sin terminal** al abrir, **instalador Windows NSIS**, desinstalación completa.
6. "Que no se rompa nunca": errores degradan, no crashean.

## Ubicaciones

- **Proyecto**: `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs`
- **Instalador**: `target\release\bundle\nsis\ZapZap_1.0.0_x64-setup.exe` (~2.83 MiB)
- **Config runtime**: `%APPDATA%\app.zapzap.desktop\config.json`
  (camelCase, tolerante a BOM: `api{enabled,port,apiKey,exposeNetwork,webhookUrl,webhookSecret}` + `accounts[{id,label,enabled}]`)
- **Perfiles WebView2**: `%APPDATA%\app.zapzap.desktop\webview\<account-id>`
- **ZapZap original (referencia, NO tocar)**: `C:\Users\PC\Desktop\Projects\Zapdude\zapzap-main`
- **tauri CLI prebuilt**: `zapdude-rs\tools\tauri.exe` (usar: `.\tools\tauri.exe build --bundles nsis`)

## Arquitectura actual (post-refactor, commit 2a847c5)

**UNA ventana "main"** (Tauri 2, feature `unstable`, multi-webview):
- Webview `rail` (76px lógico, izquierda): `rail.html` servido POR el API server.
  Logo ⚡, avatares circulares por cuenta (anillo verde=activa, punto: verde=conectado,
  naranja=espera QR, gris=cargando), botón "+" (crea cuenta sin formulario,
  id auto `account-N`, label `Cuenta N`), clic derecho en avatar = eliminar (con confirm),
  abajo ⚙️ (abre ventana settings tab api) e ⓘ (tab about). Poll cada 4s + healthWatch
  (si el server cae muestra "Reconectando…" y se auto-recarga).
- Webview `welcome`: `welcome.html` — onboarding/pantalla por defecto.
- Webviews `account-<id>`: WhatsApp Web (`https://web.whatsapp.com`), uno visible a la
  vez (`show_content`), cada uno con:
  - `data_directory` aislado (perfil propio por cuenta)
  - UA Chrome 156.0.8078.49 + `client_hints_fix.js` (identidad JS para llamadas)
  - `api_page_bridge.js` (puente `window.__zapZapApiV1`, beacon `#__zapdude=` + eval con callback)
  - `hide_download_banners.js` (oculta promos de descarga ES/EN con MutationObserver)
  - Permisos auto-Allow: Microphone, Camera, DisplayCapture, Notifications, ClipboardRead, PointerLock
  - BROWSER_ARGS: `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows`
  - Poller de beacons por cuenta (hilo 30ms)
- **Ventana "settings"** separada (WebviewWindow, `settings.html?tab=api|agent|about`):
  Configuración API (enabled/port/apiKey/webhook), Agente de IA (docs de endpoints),
  Acerca de (crédito Rafael Tosta, AGPL-3.0).
- Cerrar ventana main = ocultar (tray mantiene la app viva). Settings cierra normal.
- Tray: "Mostrar <cuenta>" por cuenta, "Ocultar ventanas", "Salir de ZapZap".
  Crear webview desde tray → hilo aparte (deadlock WebView2 en main thread).

**Clave del diseño**: el **API server sirve la propia UI** (`GET /` → rail.html,
`GET /welcome`, `GET /settings` — embebidos con include_str!). Rail/panel hacen
`fetch` **mismo-origen** → imposible fallar por CORS/IPC/file://. NO se usa Tauri IPC
para la UI (los comandos de `app_commands.rs` siguen registrados pero la UI usa HTTP).

**API server (axum, `src/api/server.rs`)** — SIEMPRE corre en `127.0.0.1:<puerto>` (default 29977):
- Management (sin auth, para la UI local): `/api/healthz`, `GET/POST /api/config`,
  `GET /api/accounts` (incluye `active` + `session_state` por cuenta, snapshots
  concurrentes con timeout 1.2s via JoinSet), `POST /api/accounts` (crea sin body,
  id auto), `DELETE /api/accounts/{id}`, `POST /api/accounts/{id}/open|hide`.
- UI endpoints: `POST /api/ui/settings?tab=` (abre ventana settings), `POST /api/ui/quit`.
- Automatización (requieren `X-API-Key`/Bearer + `enabled:true`): `/api/status`,
  `/api/message`, `/api/open-chat`, `/api/chats`, `/api/events`, `/api/events/stream` (SSE), `/api/webhook`.
- `post_config` responde PRIMERO y reinicia el server en un hilo (250ms después),
  luego `reload_shell_ui` re-apunta rail/welcome/settings al nuevo puerto.
- `SERVER: OnceLock<Mutex<Option<ServerHolder>>>` → shutdown/restart funcionan
  (el bug anterior: OnceLock no se podía re-crear tras shutdown).
- axum 0.8: rutas PLANAS `/api/...` (`.nest("/")` NO existe — pánico en runtime).
  Route params `{id}` (no `:id`).

## Historial de fixes importantes (por qué pasaron)

1. axum 0.8 `.nest("/")` pánico "Nesting at the root" → rutas planas.
2. WebView2 `ExecuteScript` llega **doble-codificado** cuando el valor es String →
   `evaluate()` desenrolla una vez (sin esto todo caía a defaults silenciosos).
3. `session_state` honesto a 3 estados: detecting/scan_qr/logged_in.
4. UI anterior usaba fetch→server no corriendo (enabled=false) + CORS file:// →
   "Failed to fetch" SIEMPRE. Fix: server siempre corre + UI servida por él.
5. UI anterior usaba `window.__TAURI__` IPC que fallaba silencioso (console.error)
   → botones muertos. Fix: todo por HTTP mismo-origen con errores visibles.
6. Ventana principal: antes file:// con resolución de paths frágil → ahora http://127.0.0.1.
7. `windows_subsystem = "windows"` en main.rs (sin consola).
8. Rate limit 60/min, body 1MB, ring 200 eventos, SSE keep-alive 15s.
9. 14 tests unitarios (`cargo test`) SIEMPRE en verde antes de entregar.

## Commits (en orden)

- `d08eb65` feat: initial Rust rewrite (Tauri 2) — multi-account, tray, API, calls identity
- `25c819e` fix: GUI app without console + human interface window
- `58d013d` fix: app loads UI without crashing + config change restarts API server
- `5dfdb65` feat: Apple-style UI + robust API port detection + add account fix
- `4f09f4f` feat(shell): ZapZap-style single window with account rail and embedded WhatsApp Web
- `2a847c5` fix(shell): serve UI from the loopback API (same-origin fetch), one-click account add works end to end

## Estado de verificación (lo ya probado automatizadamente)

- Build dev + release + NSIS: OK. Tests 14/14 OK.
- Smoke HTTP completo: healthz 200, config GET/POST, accounts CRUD,
  open/hide (cambia `active`), settings window abre, quit termina el proceso.
- App debug corre estable, sin pánicos, sin terminal.

## PENDIENTE (el usuario reporta errores, enviará captura)

- Diagnosticar con la captura del usuario los errores que ve en la UI.
- Sospechas a revisar si el "+" sigue sin funcionar en el INSTALADO:
  1) ¿Instaló el instalador NUEVO (2.83 MiB) o quedó el viejo?
  2) Puerto ocupado por otra instancia vieja de zapdude.exe
     (`Get-Process zapdude` / `netstat -ano | findstr 29977`).
  3) El rail muestra badge "Reconectando…" si el server no arrancó
     (ej: puerto en uso) — pedir esa info.
  4) Assets embebidos: cambiar ui/*.html requiere RECOMPILAR (include_str!).
- Desinstalación completa: `deleteAppDataOnUninstall` NO se llegó a añadir a
  tauri.conf.json (quedó pendiente en el refactor). Añadir bajo
  `bundle.windows.nsis: { "deleteAppDataOnUninstall": true }`.
- El usuario pidió (para después del diagnóstico): estilo aún más pro (Apple-like),
  tabs adicionales (Canales, favoritos, mute, descargas, donar) — pendiente decidir alcance.
- Llamadas/pantalla y QR real requieren prueba manual del usuario.

## Comandos útiles

```powershell
# Build + tests + instalador
cd C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
cargo build; cargo test
.\tools\tauri.exe build --bundles nsis   # instalador en target\release\bundle\nsis

# Matar instancias previas antes de rebuild (error os error 5 si no)
Get-Process -Name zapdude | Stop-Process -Force

# Correr debug con logs
Start-Process .\target\debug\zapdude.exe -RedirectStandardOutput out.log -RedirectStandardError err.log

# Smoke API
curl http://127.0.0.1:29977/api/healthz
curl http://127.0.0.1:29977/api/accounts
curl -X POST http://127.0.0.1:29977/api/accounts   # crear cuenta (como el "+")
```

## Notas de entorno

- PowerShell: cargo/tauri escriben a stderr → ruido `NativeCommandError` cosmético;
  capturar con `$o = cmd 2>&1; $LASTEXITCODE`.
- `curl.exe` (no alias de PS). Para POST con body JSON usar archivos o Invoke-RestMethod.
- git: `C:\Program Files\Git\bin\git.exe` (PATH puede no estar refrescado).
- Usuario habla español; responder en español.
