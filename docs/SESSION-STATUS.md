# ESTADO DE LA SESIÓN — ZapDude (Rust)

> Documento de continuidad para agentes/compaction. Última actualización: 2026-10-28 (noche, v3).
>
> **SIGUIENTE PASO INMEDIATO**: el usuario va a COMPACTAR la sesión y luego enviará
> MÁS FEEDBACK (dijo: "ahorita te doy más feedback"). Estar atento a capturas.
>
> La app se llama **ZapDude** (el usuario la renombró; mantener crédito a Rafael
> Tosta por ZapZap). Todo verificado con capturas de ventana reales (PrintWindow
> + PW_RENDERFULLCONTENT). Instalador actual:
> `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\target\release\bundle\nsis\ZapDude_1.0.0_x64-setup.exe`
> (~2.84 MiB).

## Estado actual del usuario (importante)

- En su máquina quedó UNA cuenta: `account-1` ("Cuenta 1", sin loguear, muestra QR).
- El usuario tenía la cuenta "default" LOGUEADA (vio su lista de chats real
  funcionando) pero la BORRÓ durante pruebas en vivo — el perfil WebView2 de esa
  sesión sigue en `%APPDATA%\app.zapzap.desktop\webview\default` (huérfano: si
  volviera a existir un account "default" recuperaría esa sesión).
- Las capturas mostraron: WhatsApp logueado renderiza perfecto, cambio de cuenta
  <350ms, drawer de opciones funcional, promo card de descarga eliminado.

## Qué es este proyecto

Reescritura completa en **Rust** del cliente **ZapZap** (original de **Rafael Tosta**,
Python/GTK, https://github.com/rafatosta/zapzap, GPL-3.0). Este fork se distribuye
como **AGPL-3.0** (obra derivada, §13) con crédito obligatorio a Rafael Tosta
(see `NOTICE.md`).

Requisitos del usuario (en orden de importancia):
1. **Multi-cuenta vital**: todas las cuentas de WhatsApp en UNA app, notificaciones
   unificadas. Experiencia EXACTAMENTE como ZapZap original: rail lateral con
   avatares, click en "+" agrega cuenta SIN FORMULARIOS → QR → escanear → entrar.
   El usuario dijo: "hazlo en un principio igual [al original] y si quieres a
   partir de ahí lo mejoras".
2. **Interfaz humana** profesional — "más premium, más profesional" — que la gente
   quiera usarlo.
3. **API local** en loopback para humanos y **agentes de IA** (REST + SSE + webhook HMAC).
4. **Llamadas/vídeo/pantalla** funcionando (identidad Chrome 156 en JS).
5. **Sin terminal** al abrir, **instalador Windows NSIS**, desinstalación completa.
6. "Que no se rompa nunca": errores degradan, no crashean.

## Ubicaciones

- **Proyecto**: `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs`
- **Instalador**: `target\release\bundle\nsis\ZapDude_1.0.0_x64-setup.exe` (~2.84 MiB)
- **Config runtime**: `%APPDATA%\app.zapzap.desktop\config.json`
  (camelCase, tolerante a BOM: `api{enabled,port,apiKey,exposeNetwork,webhookUrl,webhookSecret}` + `accounts[{id,label,enabled}]`)
- **Perfiles WebView2**: `%APPDATA%\app.zapzap.desktop\webview\<account-id>`
- **ZapZap original (referencia, NO tocar)**: `C:\Users\PC\Desktop\Projects\Zapdude\zapzap-main`
- **tauri CLI prebuilt**: `zapdude-rs\tools\tauri.exe` (usar: `.\tools\tauri.exe build --bundles nsis`)

## Arquitectura actual (v3 — drawer + position-only parking)

**UNA ventana "main"** (Tauri 2, feature `unstable`, multi-webview):
- Webview `rail` (76px lógico, izquierda): `rail.html` servido POR el API server.
  Solo: **logo ZapDude (rayito SVG) arriba** → **avatares** (squircles 44px con el
  rayito ZapDude — SIN letras/números; activa = anillo verde + glow; dot de estado:
  verde=conectada, naranja=espera QR, gris=cargando) → **botón "+"** abajo.
  Sin footer, sin tuerquita, sin ⓘ, sin versión, sin divisoria (todo eso vive en
  el drawer). Poll cada 2.5s con **diff incremental de DOM** (renderRail por
  dataset.id — sin flicker).
  - **Logo = toggle drawer de opciones** (POST /api/ui/drawer).
  - Click en avatar = abrir/cambiar a esa cuenta.
  - **Eliminar cuenta**: clic derecho sobre el avatar lo "arma" (rojo ×, pulso) →
    clic de nuevo elimina; Esc o clic en otro lo desarma (SIN auto-desarme de
    tiempo — un timer de 4s se comía los clicks lentos y "no dejaba borrar").
- Webview `welcome`: backdrop técnico (NUNCA es destino de ningún flujo).
- **Webview `drawer`** (LAZY, se crea al primer toggle para quedar ENCIMA de las
  cuentas): `drawer.html` servido por el server — panel de 380px sobre el contenido
  con secciones separadas: **Configuración API** (toggle + badge Activa/Desactivada,
  puerto, clave, webhook, guardar), **Agente de IA** (endpoints), **Acerca de**
  (ZapDude, AGPL-3.0, crédito Rafael Tosta). Cierra con X o Esc. La ventana
  "settings" separada quedó en el código PERO la UI ya no la usa.
- Webviews `account-<id>`: WhatsApp Web (`https://web.whatsapp.com`), cada uno con:
  - `data_directory` aislado (perfil propio por cuenta)
  - UA Chrome 156.0.8078.49 + `client_hints_fix.js` (identidad JS para llamadas) —
    inyectado en document-start (startup script, pequeño)
  - `api_page_bridge.js` — **INYECTADO DIFERIDO** (ver "Lección crítica")
  - `hide_download_banners.js` — **REESCRITO SEGURO** (ver "Lección crítica")
  - Permisos auto-Allow: Microphone, Camera, DisplayCapture, Notifications, ClipboardRead, PointerLock
- **VISIBILIDAD (modelo parked offscreen, position-only)**: el webview activo va
  en `(RAIL_WIDTH, 0)` a tamaño completo; los inactivos van a **x=-24000
  (PARKED_X) TAMBIÉN A TAMAÑO COMPLETO**. El switch es SOLO movimiento de
  posición → WhatsApp NO re-renderiza → cambio instantáneo (<350ms medidos).
  (El modelo anterior 1×1 forzaba re-render completo de WhatsApp: 2-3s por switch.)
- **Arranque (orden crítico en main.rs setup)**:
  1. `enforce_single_instance()` (mutex `Local\app.zapzap.desktop.single`; 2ª
     instancia sale inmediatamente — evita peleas de puerto/config)
  2. `start_loopback_server()` — **ANTES de crear webviews** (antes la UI cargaba
     antes de que el server existiera → error Edge "can't reach this page" al abrir)
  3. `create_shell` (rail + welcome)
  4. `ensure_account_webview` por cuenta (nacidos a TAMAÑO COMPLETO y luego
     parqueados; un webview nacido 1×1 u oculto queda gris para siempre)
  5. `auto_open_first_account` — abre la primera cuenta directo (sin welcome
     interstitial; crea `Cuenta 1` si no hay ninguna — comportamiento ZapZap original)
  6. tray
- Carrera "already exists": `window.add_child` se despacha async al hilo principal;
  si `add_child` responde "already exists" se trata como éxito (no fallar el open).
- Ventana "settings" separada (`settings.html?tab=api|agent|about`), servida por el server.
- Cerrar ventana main = ocultar (tray mantiene la app viva).
- Tray: "Mostrar <cuenta>" por cuenta, "Ocultar ventanas", "Salir de ZapDude".
- API server: bind con RETRY (40 × 250ms) — el restart tras `POST /api/config`
  fallaba con error 10048 (puerto aún ocupado por el listener viejo).
- `list_accounts` responde AL INSTANTE con caché de session_state (TTL 2.5s,
  refresco en background con tokio::spawn) — el rail ya no espera 1.2s de snapshots.

**Clave del diseño**: el **API server sirve la propia UI** (`GET /` → rail.html,
`GET /welcome`, `GET /settings`, `GET /drawer` — embebidos con include_str!). Todas
las páginas hacen `fetch` **mismo-origen** → imposible fallar por CORS/IPC/file://.
NO se usa Tauri IPC para la UI (los comandos de `app_commands.rs` siguen
registrados pero la UI usa HTTP).

## ⚠️ LECCIÓN CRÍTICA — la "página gris" (2 causas encadenadas)

Síntoma: rail/welcome perfectos; al abrir una cuenta, el área de contenido queda
en gris plano, PERO el DOM de WhatsApp está vivo (los snapshots del bridge veían
el QR — `session_state: scan_qr`). Verificado con capturas PrintWindow
(PW_RENDERFULLCONTENT=2) en `C:\temp\*.png`.

**Causa raíz 1 — webviews creados ocultos o nacidos enano quedan grises para
siempre.** WebView2 (child webviews en wry 0.57) solo arranca su pipeline de
composición si nace a tamaño real en el slot de contenido. Fix: nacer a tamaño
completo + parqueo por POSICIÓN (x=-24000, tamaño completo — ver Arquitectura).
NOTA: al inicio se creyó que el parking offscreen también causaba gris y se pasó
por 1×1; después de arreglar la causa raíz 2 se comprobó que el parking offscreen
funciona perfecto y es el que da switches instantáneos.

**Causa raíz 2 — `hide_download_banners.js` (versión vieja) borraba la página.**
Su barrido genérico marcaba `__zapdudeChecked` por nodo y ocultaba cualquier
contenedor cuyo `textContent` coincidiera con el patrón de promo; los
contenedores grandes del landing coinciden (su texto contiene "Descarga WhatsApp
para Windows") → TODA la página quedaba `display:none` → fondo oscuro plano.
Correlación engañosa: el bridge solo parecía "vivo" en las corridas grises
(por el orden en que cada script se instalaba/truncaba). Fix: reescritura
"smallest matching node only" + colapso controlado de ancestros (máx 6 niveles,
solo si TODOS los hijos quedan display:none computado, contenedores de ≤4 hijos).

**Hallazgos colaterales del debug (empíricos, no tocar sin re-testear)**:
- Pasar `additional_browser_args` custom en child webviews + UA Chrome ⇒ gris
  (probado: full set, solo ms*, solo bg-flags, wry-default explícito — TODOS grises
  con UA; sin args custom ⇒ pinta). Por eso NO se pasan args custom en producción.
- `BROWSER_ARGS` viejo usaba `\` de continuación de Rust que pega tokens
  ("...Protection--disable-...") — malformado además.
- Un solo initialization script combinado de ~23 KB también se asoció al gris;
  los scripts ahora van separados: startup = client_hints (5.5KB); bridge+promos
  van DIFERIDOS vía eval idempotente desde el poller de beacons
  (`DEFER_INSTALL_EVERY_TICKS=100` ≈ cada 3s, chequeo `!window.__zapZapApiV1`).
- El transporte de eventos push (hook `window.Notification` + observer chat_list)
  quedó DESHABILITADO en el bridge (era parte del enredado gris). La API pasiva
  (status/chats/send/open-chat) funciona. Eventos push: pendiente de rediseño seguro.

## Debug harness (reutilizable)

- Captura de ventana con contenido WebView2: PrintWindow + `PW_RENDERFULLCONTENT=2`
  (los webviews son DirectComposition; sin ese flag sale vacío). Scripts en el
  historial; guardar en `C:\temp\*.png` y leerlas con visión.
- Bisección por env var `ZAPDUKE_WEBVIEW_OPTS` (window.rs `has()`):
  `ua|scripts|data|args|perms|scripts1|scripts2|scripts3|argsbg|argsms|argsry` —
  dejar el harness; es barato y ya salvó la sesión dos veces.

## Historial de fixes importantes (por qué pasaron)

1. axum 0.8 `.nest("/")` pánico "Nesting at the root" → rutas planas.
2. WebView2 `ExecuteScript` llega **doble-codificado** cuando el valor es String →
   `evaluate()` desenrolla una vez (sin esto todo caía a defaults silenciosos).
3. `session_state` honesto a 3 estados: detecting/scan_qr/logged_in.
4. UI anterior usaba fetch→server no corriendo + CORS file:// → "Failed to fetch".
   Fix: server siempre corre + UI servida por él (mismo origen).
5. UI anterior usaba `window.__TAURI__` IPC que fallaba silencioso → botones muertos.
6. `windows_subsystem = "windows"` en main.rs (sin consola).
7. Rate limit 60/min, body 1MB, ring 200 eventos, SSE keep-alive 15s.
8. 14 tests unitarios (`cargo test`) SIEMPRE en verde antes de entregar.
9. (HOY) Server ANTES de shell; auto-open primera cuenta; nacer a tamaño completo;
   parqueo offscreen position-only (switch instantáneo); "already exists" no fatal;
   hide-banners reescrito smallest-node + zonas protegidas; eventos push del bridge
   desactivados; bind con retry; single-instance mutex; drawer de opciones en la
   misma ventana; caché de session_state; deleteAppDataOnUninstall no soportado
   por el tauri-build local → NSIS hook `installerHooks: ./windows/installer-hooks.nsh`
   (RMDir /r del data dir al desinstalar).

## Commits (en orden)

- `d08eb65` feat: initial Rust rewrite (Tauri 2) — multi-account, tray, API, calls identity
- `25c819e` fix: GUI app without console + human interface window
- `58d013d` fix: app loads UI without crashing + config change restarts API server
- `5dfdb65` feat: Apple-style UI + robust API port detection + add account fix
- `4f09f4f` feat(shell): ZapZap-style single window with account rail and embedded WhatsApp Web
- `2a847c5` fix(shell): serve UI from the loopback API (same-origin fetch), one-click account add works end to end
- `4602fd9` docs: session status handoff (architecture, fixes, pending diagnostics)
- `a868fe9` fix(shell): server-first startup, parked 1x1 webviews, safe promo hider, single instance
- `4ca7ad9` feat(ui): rename to ZapDude, premium rail with bolt avatars, delete-any-account flow
- `883e062` feat(ui): logo opens in-window options drawer, position-only parking makes account switching instant

## Round 3 (feedback: logo-menus, velocidad, parpadeo, footer)

- **Drawer de opciones en la MISMA ventana** (ui/drawer.html, label `drawer`,
  DRAWER_WIDTH=380): el logo ZapDude del rail lo abre/cierra
  (POST /api/ui/drawer → window::toggle_drawer, creado LAZY la primera vez para
  que quede ENCIMA de las cuentas). Secciones separadas: Configuración API
  (form + badge Activa/Desactivada + guardar), Agente de IA (endpoints), Acerca de
  (ZapDude + AGPL + crédito Rafael Tosta). Cierra con X o Esc.
- **Rail sin footer**: solo logo → avatares → "+". Sin tuerquita, sin ⓘ, sin
  versión, sin la línea divisoria pegada al "+".
- **Switch INSTANTÁNEO**: las cuentas inactivas van parqueadas a TAMAÑO COMPLETO
  en x=-24000 (PARKED_X) y el switch es SOLO movimiento de posición — WhatsApp no
  re-renderiza (con 1×1 re-renderizaba completo = 2-3s). Medido: pintado completo
  a 350ms. El viejo "gray al parkear offscreen" era el script de promos (ya
  reescrito) — el parking offscreen funciona bien con los scripts arreglados.
- **Rail sin parpadeo**: renderRail() hace diff incremental (por dataset.id,
  insertBefore en orden) en vez de innerHTML="" por cada poll (2.5s) o add.
- open_account_webview cierra el drawer (el QR/chat toma todo el área).
- reload_shell_ui re-apunta también el drawer (`/drawer`).
- Config de la ventana "settings" y su endpoint /api/ui/settings quedan en el
  código pero la UI ya no los usa (todo vive en el drawer).

## Round 2 (feedback del usuario tras probar: renombrado + premium + delete)

- **Nombre**: ZapDude en TODAS partes (ventana "ZapDude", "Configuración — ZapDude",
  tray "Salir de ZapDude", `productName` → instalador `ZapDude_1.0.0_x64-setup.exe`).
  identifier/`app.zapzap.desktop` NO cambia (preserva config).
- **Rail premium** (ui/rail.html): SVG symbol library (bolt/plus/gear/info — cero
  emojis), logo squircle con glow que **abre la cuenta activa** (antes no hacía
  nada), TODOS los avatares muestran el rayito (adiós letras/números D/2), anillo
  verde + glow en la activa, dot de estado, tooltips glass, fade masks en el scroll.
- **Delete flow**: sin auto-desarme (un timer de 4s se comía clicks lentos — por eso
  "no deja borrar la primera"); right-click arma, click de nuevo elimina, Esc desarma.
  `sanitize_accounts` permite lista VACÍA (adiós default zombi). Al eliminar la
  cuenta en pantalla → `close_account_webview` → `auto_open_first_account` (abre la
  primera restante o CREA "Cuenta 1" con QR). El welcome NUNCA es destino.
- **Velocidad**: `list_accounts` responde AL INSTANTE con caché de session_state
  (TTL 2.5s, refresco background con tokio::spawn) — el rail ya no espera 1.2s de
  snapshots. Switch medido: <500ms con render completo (capturas).
- **Promo card eliminado completo** del panel de chat vacío: patrones ampliados
  (subtítulo "obtén funciones adicionales / get extra features", botón exacto
  "Descargar/Download" solo si es hoja con texto ≤12 chars), zonas protegidas
  (#pane-side, [role=listitem], [contenteditable], #main — NUNCA se tocan),
  colapso de ancestros solo si TODOS los hijos son promo/icono sin texto.
- El usuario probó EN VIVO durante el desarrollo: borró todas las cuentas y la app
  auto-creó "Cuenta 1" con QR (el flujo nuevo funcionando en producción real).

## Estado de verificación (con capturas reales de ventana)

- ARRANQUE: sin error de Edge (server primero); QR de WhatsApp visible desde el
  primer instante; cuenta activa con anillo.
- SWITCH de cuentas D→2→D: sin gris, sin recarga, instantáneo (modelo parked 1×1).
- "+": crea cuenta Y abre su QR al instante (account-4 verificado con captura).
- DELETE por clic-derecho-armado: ok (sin overlays recortados).
- Guard de instancia única: 2ª instancia sale sola; la original sigue viva.
- QUIT desde API: proceso termina limpio; config intacta (ids preservados).
- Banners: título "Descarga WhatsApp para Windows" oculto de forma segura
  (puede quedar la tira del promo con icono+botón en el landing del QR — solo se
  ve antes de escanear; afinar después inspeccionando el DOM real).
- Tests 14/14. Build release + NSIS OK (`ZapDude_1.0.0_x64-setup.exe`, 2.84 MiB).
- (Round 3) Drawer abre/cierra sobre el contenido; rail solo logo→cuentas→"+";
  switch instantáneo con parking position-only; rail sin flicker (diff DOM).

## PENDIENTE (el usuario compactará y enviará MÁS FEEDBACK — estar atento a capturas)

- **Más feedback del usuario** tras la compactación — es la prioridad inmediata.
- Prueba de instalación limpia del `ZapDude_1.0.0_x64-setup.exe` (desinstalar el
  viejo; el hook NSIS borra `%APPDATA%\app.zapzap.desktop` al desinstalar).
- Rediseñar los eventos push (notificaciones → API/webhook) de forma segura
  (sin hooks globales frágiles): idealmente vía CDP/WebMessageReceived o polling
  del host en vez de MutationObservers. Hoy la API pasiva funciona
  (status/chats/send/open-chat) pero SIN eventos espontáneos.
- Iconos del rail estilo original (llamadas, favoritos, canales, etc.) y tabs
  adicionales (Canales, favoritos, mute, descargas, donar) — el usuario aún no
  lo pide formalmente para esta ronda; decidir alcance tras su feedback.
- Llamadas/pantalla requieren prueba manual del usuario (identidad Chrome 156
  ya activa con UA + client hints).
- Limpieza menor: ventana "settings" + endpoint `/api/ui/settings` +
  `app_commands.rs` quedaron sin uso por la UI (código muerto inofensivo;
  remover en una pasada de limpieza si se quiere).

## Comandos útiles

```powershell
# Build + tests + instalador
cd C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
cargo build; cargo test
.\tools\tauri.exe build --bundles nsis   # instalador en target\release\bundle\nsis

# Matar instancias previas antes de rebuild (error os error 5 si no)
Get-Process -Name zapdude | Stop-Process -Force

# Limpiar SIEMPRE la env var de bisección antes de probar producción
Remove-Item Env:ZAPDUKE_WEBVIEW_OPTS -ErrorAction SilentlyContinue

# Smoke API
curl http://127.0.0.1:29977/api/healthz
curl http://127.0.0.1:29977/api/accounts
curl -X POST http://127.0.0.1:29977/api/accounts   # crear cuenta (como el "+")
```

## Notas de entorno

- PowerShell: cargo/tauri escriben a stderr → ruido `NativeCommandError` cosmético;
  capturar con `$o = cmd 2>&1; $LASTEXITCODE`.
- **El mismo shell de PowerShell PERSISTE env vars entre comandos** — la env var de
  debug `ZAPDUKE_WEBVIEW_OPTS` contaminó varias corridas; limpiarla SIEMPRE.
- `curl.exe` (no alias de PS). Para POST con body JSON usar archivos o Invoke-RestMethod.
- git: `C:\Program Files\Git\bin\git.exe` (PATH puede no estar refrescado).
- Usuario habla español; responder en español.

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
