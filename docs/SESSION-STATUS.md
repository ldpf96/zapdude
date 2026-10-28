# ESTADO DE LA SESIÓN — Zapdude (Rust)

> Documento de continuidad para agentes/compaction. Última actualización:
> 2026-10-28 (tarde, v7).
>
> **SIGUIENTE PASO INMEDIATO**: el usuario está PROBANDO en vivo la versión
> nueva (binario de 2026-10-28 14:56, ya instalado). Round 7 de feedback
> ATENDIDO Y VERIFICADO: (a) icono viejo en el exe reemplazado por el
> verde/rayito negro, (b) cierre del drawer instantáneo (1 ms), (c) el
> renombrado ya no borra lo que escribes. Pendiente: confirmación del
> usuario de que ve el icono verde en la bandeja/taskbar (ie4uinit -show ya
> corrido), prueba real del pop-out de llamadas y del QR escaneado.
>
> La app se llama **Zapdude** (Z mayúscula, resto minúscula — aclaración
> EXPLÍCITA del usuario: "Debe ser Zapdude, no ZapDude"; NO usar "ZapDude").
> **Marco legal/atribución (decisión del usuario, v7)**: Zapdude es un
> PROYECTO ORIGINAL de **ldpf96** (su GitHub) con AGPL-3.0-or-later; el
> código es 100% nuevo (no copia/portea nada de ZapZap). La *idea* está
> INSPIRADA en ZapZap (Rafael Tosta) y así se menciona en README/NOTICE/
> drawer, pero NUNCA como "fork/obra derivada/reescritura" ni "crédito
> obligatorio". HEADER de red renombrado: `X-Zapdude-Signature`
> (antes `X-ZapZap-Signature`) y UA `Zapdude-API/1`; flag de bridge
> `window.__zapdudeApiV1` (antes `__zapZapApiV1`). El identifier de config
> `app.zapzap.desktop` NO cambia (preserva config del usuario). Todo
> verificado con capturas de ventana reales (PrintWindow +
> PW_RENDERFULLCONTENT). Instalador actual:
> `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\target\release\bundle\nsis\Zapdude_1.0.0_x64-setup.exe`
> (2,943,632 bytes, ~2.81 MiB).

## Estado actual del usuario (importante, v7)

- El usuario tiene 6 cuentas creadas (account-1..6, labels "Cuenta 1".."Cuenta 6")
  y está probando EN VIVO: tema claro, bandeja, menú X, "+" del rail, renombrar.
- App corriendo con el binario nuevo: `C:\Users\PC\AppData\Local\Zapdude\zapdude.exe`
  (release, 11,274,752 bytes, 2026-10-28 14:56). Title de la ventana "Zapdude" +
  icono verde/rayito negro VERIFICADO leyendo el HICON real (WM_GETICON →
  GetIconInfo → GetObject + samples de píxeles: 99,222,154 / 4,35,26).
- Theme toggle verificado uniforme por el usuario ("perfecto, puedes
  spamearlo"). Rail/drawer/bienvenida bien.
- El usuario dijo en Round 7: estructura bonita, todo bien bonito; la queja
  dura era el ICONO GRIS CON RAYO BLANCO (por eso Round 7 lo mata).

## Qué es este proyecto

**Zapdude** = cliente de escritorio de WhatsApp Web multi-cuenta con API
local, **código original de ldpf96** licenciado **AGPL-3.0-or-later**
(LICENSE ya es el texto AGPL-3.0). La *idea* está INSPIRADA en el concepto
del cliente **ZapZap** (Rafael Tosta, https://github.com/rafatosta/zapzap,
Python/GTK) que sí es GPL: NO se reutiliza su código (stacks distintos,
escritura desde cero), así que NO es obra derivada y no hereda
obligaciones GPL — el crédito de inspiración es cortesía, no legal. NO
redactar como "fork/reescritura/obra derivada". Ver NOTICE.md.

Requisitos del usuario (en orden de importancia):
1. **Multi-cuenta vital**: todas las cuentas de WhatsApp en UNA app, notificaciones
   unificadas. Experiencia EXACTAMENTE como el cliente que lo inspiró: rail
   lateral con avatares, click en "+" agrega cuenta SIN FORMULARIOS → QR →
   escanear → entrar.
2. **Interfaz humana** profesional — "más premium, más profesional" — que la gente
   quiera usarlo.
3. **Nombre del autor visible**: el usuario quiere SU nombre (ldpf96) en
   los créditos.
4. **API local** en loopback para humanos y **agentes de IA** (REST + SSE + webhook HMAC).
5. **Llamadas/vídeo/pantalla** funcionando (identidad Chrome 156 en JS).
6. **Sin terminal** al abrir, **instalador Windows NSIS**, desinstalación completa.
7. "Que no se rompa nunca": errores degradan, no crashean.
8. **Publicar en GitHub** (repo `ldpf96/...` probable; usuario interesado).

## Ubicaciones

- **Proyecto**: `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs`
- **Instalador**: `target\release\bundle\nsis\Zapdude_1.0.0_x64-setup.exe` (~2.81 MiB)
- **Config runtime**: `%APPDATA%\app.zapzap.desktop\config.json`
  (camelCase, tolerante a BOM: `api{enabled,port,apiKey,exposeNetwork,webhookUrl,webhookSecret}` + `accounts[{id,label,enabled}]` + `prefs{theme,autostart,startMinimized}`)
- **Perfiles WebView2**: `%APPDATA%\app.zapzap.desktop\webview\<account-id>`
- **ZapZap original (referencia, NO tocar)**: `C:\Users\PC\Desktop\Projects\Zapdude\zapzap-main`
- **tauri CLI prebuilt**: `zapdude-rs\tools\tauri.exe` (usar: `.\tools\tauri.exe build --bundles nsis`)
- **Generador de iconos**: `scripts/regen-icons.ps1` (reconstruye icon.ico
  desde los PNG aprobados + re-renderiza icon.png/tiles UWP);
  auditor de .ico: `tools/dump-ico.ps1` (extrae los frames para verlos).

## Arquitectura actual (v3 — drawer + position-only parking)

**UNA ventana "main"** (Tauri 2, feature `unstable`, multi-webview):
- Webview `rail` (76px lógico, izquierda): `rail.html` servido POR el API server.
  Solo: **logo Zapdude (rayito SVG) arriba** → **avatares** (squircles 44px con el
  rayito Zapdude — SIN letras/números; activa = anillo verde + glow; dot de estado:
  verde=conectada, naranja=espera QR, gris=cargando) → **botón "+"** abajo.
  Sin footer, sin tuerquita, sin ℹ, sin versión (todo eso vive en el drawer).
  Poll cada 2.5s con **diff incremental de DOM** (renderRail por dataset.id — sin flicker).
  - **Logo = toggle drawer de opciones** (POST /api/ui/drawer).
  - Click en avatar = abrir/cambiar a esa cuenta.
  - **Eliminar cuenta**: clic derecho sobre el avatar lo "arma" (rojo ×, pulso) →
    clic de nuevo elimina; Esc o clic en otro lo desarma (SIN auto-desarme de
    tiempo — un timer de 4s se comía los clicks lentos y "no dejaba borrar").
- Webview `welcome`: backdrop técnico (NUNCA es destino de ningún flujo).
- **Webview `drawer`** (LAZY, creado en el primer toggle para quedar ENCIMA de
  las cuentas): `drawer.html` servido por el server — panel a pantalla completa
  con secciones: **Preferencias** (tema claro, autostart, inicio minimizado),
  **Cuentas** (renombrar con refresco incremental — ver Round 7), **Configuración
  API** (toggle + badge, puerto, clave, webhook, guardar), **Agente de IA**
  (endpoints), **Acerca de** (Zapdude, AGPL-3.0, autor ldpf96, inspiración
  ZapZap). Cierra
  con X o Esc (el cierre ahora es INSTANTÁNEO — relayout directo). La ventana
  "settings" separada quedó eliminada del código.
- Webviews `account-<id>`: WhatsApp Web (`https://web.whatsapp.com`), cada uno con:
  - `data_directory` aislado (perfil propio por cuenta)
  - UA Chrome 156.0.8078.49 + `client_hints_fix.js` (identidad JS para llamadas) —
    inyectado en document-start (startup script, pequeño)
  - `api_page_bridge.js` — INYECTADO DIFERIDO (ver "Lección crítica")
  - `hide_download_banners.js` — REESCRITO SEGURO (ver "Lección crítica")
  - Permisos auto-Allow: Microphone, Camera, DisplayCapture, Notifications, ClipboardRead, PointerLock
  - `on_new_window` → pop-out REAL de llamadas (comparte el environment WebView2)
- **VISIBILIDAD (modelo parked offscreen, position-only)**: el webview activo va
  en `(RAIL_WIDTH, 0)` a tamaño completo; los inactivos van a **x=-24000
  (PARKED_X) TAMBIÉN A TAMAÑO COMPLETO**. El switch es SOLO movimiento de
  posición → WhatsApp NO re-renderiza → cambio instantáneo (<350ms medidos).
- **Arranque (orden crítico en main.rs setup)**:
  1. `enforce_single_instance()` (mutex `Local\app.zapzap.desktop.single`)
  2. `start_loopback_server()` — ANTES de crear webviews
  3. `create_shell` (rail + welcome)
  4. `ensure_account_webview` por cuenta (nacidos a TAMAÑO COMPLETO y luego
     parqueados; un webview nacido 1×1 u oculto queda gris para siempre)
  5. `auto_open_first_account` — abre la primera cuenta directo (crea "Cuenta 1"
     si no hay ninguna — comportamiento ZapZap original)
  6. tray
- Carrera "already exists": `window.add_child` se despacha async al hilo principal;
  si responde "already exists" se trata como éxito.
- Cerrar ventana main = ocultar (tray mantiene la app viva).
- Tray: "Mostrar <cuenta>" por cuenta, "Ocultar ventanas", "Salir de Zapdude";
  el menú se reconstruye al agregar/eliminar/REnombrar (refresh_tray).
- API server: bind con RETRY (40 × 250ms).
- `list_accounts` responde AL INSTANTE con caché de session_state (TTL 2.5s,
  refresco en background con tokio::spawn).

**Clave del diseño**: el **API server sirve la propia UI** (`GET /` → rail.html,
`GET /welcome`, `GET /drawer` — embebidos con include_str!). Todas las páginas
hacen `fetch` mismo-origen. NO se usa Tauri IPC para la UI.

## ⚠️ LECCIÓN CRÍTICA — la "página gris" (2 causas encadenadas)

Síntoma: rail/welcome perfectos; al abrir una cuenta, el área de contenido queda
en gris plano, PERO el DOM de WhatsApp está vivo.

**Causa raíz 1 — webviews creados ocultos o nacidos enanos quedan grises para
siempre.** WebView2 (child webviews en wry 0.57) solo arranca su pipeline de
composición si nace a tamaño real en el slot de contenido. Fix: nacer a tamaño
completo + parqueo por POSICIÓN (x=-24000, tamaño completo).

**Causa raíz 2 — `hide_download_banners.js` (versión vieja) borraba la página.**
Fix: reescritura "smallest matching node only" + colapso controlado de ancestros.

**Hallazgos colaterales del debug (empíricos, no tocar sin re-testear)**:
- Pasar `additional_browser_args` custom en child webviews + UA Chrome ⇒ gris.
  NO se pasan args custom en producción.
- Un solo initialization script combinado de ~23KB también se asoció al gris;
  los scripts van separados: startup = client_hints (5.5KB); bridge+promos van
  DIFERIDOS vía eval idempotente desde el poller de beacons (DEFER_INSTALL_EVERY_TICKS=100
  ≈ cada 3s, chequeo `!window.__zapdudeApiV1`).
- El transporte de eventos push (hook `window.Notification` + observer chat_list)
  quedó DESHABILITADO en el bridge. La API pasiva (status/chats/send/open-chat)
  funciona. Eventos push: pendiente de rediseño seguro.

## Debug harness (reutilizable)

- Captura de ventana con contenido WebView2: PrintWindow + `PW_RENDERFULLCONTENT=2`
  (los webviews son DirectComposition; sin ese flag sale vacío). Guardar en
  `C:\temp\*.png` y leerlas con visión.
- Bisección por env var `ZAPDUKE_WEBVIEW_OPTS` (window.rs `has()`):
  `ua|scripts|data|args|perms|scripts1|scripts2|scripts3|argsbg|argsms|argsry`.
- **HERRAMIENTA NUEVA (Round 7)**: medir el parking del drawer SIN tocar la UI
  enumerando los childs `WRY_WEBVIEW` de la ventana main y observando quién pasa
  de x≈+RAIL a x<-10000 (temp/opencode/measure-close2.ps1, copiada de ese
  espíritu). LEE rects aunque la ventana esté oculta en la bandeja.
- **HERRAMIENTA NUEVA (Round 7)**: leer el icono real de la ventana:
  WM_GETICON(0x7F, wp=0/1) + GetClassLongPtr(GCLP_HICON/-SM) → GetIconInfo →
  GetObject(BITMAP) para dims → Image.FromHbitmap → samples de RGB
  (temp/opencode/read-icons.ps1 + decode-icon.ps1). Funciona con la ventana
  oculta (no interrumpe al usuario).
- Tests: 14 unitarios (`cargo test`) SIEMPRE en verde antes de entregar.

## Historial de fixes importantes (por qué pasaron)

1. axum 0.8 `.nest("/")` pánico "Nesting at the root" → rutas planas.
2. WebView2 `ExecuteScript` llega doble-codificado cuando el valor es String →
   `evaluate()` desenrolla una vez.
3. `session_state` honesto a 3 estados: detecting/scan_qr/logged_in.
4. UI anterior usaba fetch→server no corriendo + CORS file:// → "Failed to fetch".
   Fix: server siempre corre + UI servida por él (mismo origen).
5. UI anterior usaba `window.__TAURI__` IPC que fallaba silencioso → botones muertos.
6. `windows_subsystem = "windows"` en main.rs (sin consola).
7. Rate limit 60/min, body 1MB, ring 200 eventos, SSE keep-alive 15s.
8. Tests SIEMPRE en verde antes de entregar.
9. Server ANTES de shell; auto-open primera cuenta; nacer a tamaño completo;
   parqueo offscreen position-only (switch instantáneo); "already exists" no fatal;
   hide-banners reescrito smallest-node + zonas protegidas; eventos push del bridge
   desactivados; bind con retry; single-instance mutex; drawer de opciones en la
   misma ventana; caché de session_state; NSIS hook `installerHooks` (RMDir /r
   del data dir al desinstalar).

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
- `b7bbee0` feat: full-screen options, lazy account loading, visible delete-X, rename to Zapdude, publish-ready
- `c6335cf` feat: light/dark theme, geometry watchdog, tray fixes, autostart, new app icon, call pop-outs, symmetric rail, account renaming
- `33280c9` fix: virgin installs create 'Cuenta 1' instead of legacy 'default'
- `61bb0c5` fix: proper green/black app icon, instant drawer close, rename-safe account list (Round 7)

## Round 3 (feedback: logo-menús, velocidad, parpadeo, footer)

- **Drawer de opciones en la MISMA ventana** (ui/drawer.html, label `drawer`):
  el logo Zapdude del rail lo abre/cierra (POST /api/ui/drawer →
  window::toggle_drawer, creado LAZY la primera vez para que quede ENCIMA de
  las cuentas). Secciones separadas: Configuración API (form + badge
  Activa/Desactivada + guardar), Agente de IA (endpoints), Acerca de (Zapdude +
  AGPL + crédito Rafael Tosta). Cierra con X o Esc.
- **Rail sin footer**: solo logo → avatares → "+". Sin tuerquita, sin ℹ, sin
  versión, sin la línea divisoria pegada al "+".
- **Switch INSTANTÁNEO**: las cuentas inactivas van parqueadas a TAMAÑO COMPLETO
  en x=-24000 (PARKED_X) y el switch es SOLO movimiento de posición — WhatsApp
  no re-renderiza (con 1×1 re-renderizaba completo = 2-3s). Medido: pintado
  completo a 350ms.
- **Rail sin parpadeo**: renderRail() hace diff incremental (por dataset.id,
  insertBefore en orden) en vez de innerHTML="" por cada poll (2.5s) o add.
- open_account_webview cierra el drawer (el QR/chat toma todo el área).
- reload_shell_ui re-apunta también el drawer (`/drawer`).
- Config de la ventana "settings" quedó en el código y luego eliminada (todo
  vive en el drawer).

## Round 4 (feedback 2: pantalla completa, lazy load, X visible, "Zapdude", publicar)

- **Opciones a PANTALLA COMPLETA**: el drawer (logo del rail) cubre TODO el área
  de contenido, columna centrada max-width 780px. El rail sigue visible para que
  el logo siga haciendo toggle. Verificado con captura (f2-drawer-fullscreen).
- **Carga diferida de cuentas** (velocidad con muchas cuentas): main.rs YA NO
  crea un webview por cuenta al arranque — solo `auto_open_first_account`; las
  demás cuentas crean su webview LAZY en el primer open. Trade-off: las cuentas
  de fondo no dan notificaciones hasta abrirse una vez por sesión (README).
- **X VISIBLE al armar con clic derecho**: rail.html añade symbol `#i-x` y
  renderRail hace `use.setAttribute('href', armed ? '#i-x' : '#i-bolt')`.
- **Divisorias curvas estéticas** en el rail: `.divider.below` (bajo el logo) y
  `.divider.above` (sobre el "+") — hairline verde con gradiente y borde curvo.
- **Menú contextual del navegador BLOQUEADO** en la UI: `contextmenu →
  preventDefault` en rail/drawer/welcome. Los webviews de WhatsApp conservan SU menú.
- **Borrar la última cuenta NO auto-crea una vacía**: nuevo
  `open_first_remaining_or_welcome` (window.rs) — abre la primera cuenta
  restante o muestra el welcome (que YA tiene botón "+ Agregar cuenta").
  El arranque (`auto_open_first_account`) SÍ crea "Cuenta 1" si no hay ninguna.
- **Renombre a "Zapdude"** (Z mayúscula, resto minúscula): productName, título
  de ventana, tray, healthz service, status name, textos UI, README, NOTICE.
  El binario sigue `zapdude.exe` (cargo name) y el identifier
  `app.zapzap.desktop` NO cambia.
- **FIX CRÍTICO del hider de banners (banner del QR)**: causa raíz TRIPLE —
  (1) el instalador diferido envolvía bridge+hider en UN SOLO eval: ahora
  `deferred_scripts()` devuelve pares (flag, script) con SU flag y SU try/catch.
  (2) El botón del promo card es un BUTTON cuyo textContent es
  "Downloadic-download" → `cleanText()` que elimina tokens `ic-*`.
  (3) El card completo se oculta con `hideLandingPromoCard()`: el card del QR
  (sin texto promo) jamás matchea. Guardias: solo corre sin `#pane-side`/`#main`
  VISIBLES (usar getClientRects).
- `withGlobalTauri: false` (no se usa IPC).

## Round 5 (feedback 3: tema, anclaje, bandeja, autostart, icono, popouts, rail simétrico)

- **DIAGNÓSTICO CLAVE**: el usuario estaba corriendo la app VIEJA instalada —
  de ahí quejas de "no sale la X" y colores viejos. Al final de la ronda se
  instala el NSIS nuevo en silencio.
- **Reset del config a "default"** (cuenta "default" zombi): CARRERA entre
  binario viejo instalado y el nuevo — el viejo leyó un config.json a medio
  escribir → parse fail → `unwrap_or_default` → persistió el default. FIX:
  `SettingsState::load` reintenta la lectura (4 intentos × 60ms).
- **TEMA CLARO/OSCURO**: sección Preferencias en el drawer con switch "Tema
  claro". Config: `AppPrefs { theme, autostart, startMinimized }` (camelCase,
  serde default — compat con configs viejos). Endpoints GET/POST /api/prefs.
  El rail re-chequea el tema cada 10s; el drawer aplica al instante. El tema
  NO toca la página de WhatsApp.
- **NAVEGADOR ANCLADO A LAS 4 ESQUINAS**: relayout también corre en
  `ScaleFactorChanged` y `Focused` (además de Resized), MÁS un watchdog de
  geometría cada 2s (thread "zapdude-geometry").
- **BANDEJA ARREGLADA**: (1) al hacer clic en "Mostrar <cuenta>" la ventana
  principal se muestra/des-minimiza/enfoca ANTES de abrir el webview;
  (2) `refresh_tray` reconstruye el menú cuando se agrega/elimina/renomina una
  cuenta; (3) clic IZQUIERDO en el icono de la bandeja trae la ventana;
  (4) "Ocultar ventanas" oculta TODAS las ventanas de verdad.
- **AUTOSTART + INICIO MINIMIZADO**: Preferencias → "Iniciar con Windows"
  (HKCU Run key vía crate winreg) y "Iniciar minimizado" (arranca sin ventana,
  vive en la bandeja). Verificado.
- **ICONO DE LA APP** (pedido: "el verde con rayito negro estaba bien bonito"):
  generados icons/*.png (16..256) + icon.ico multi-tamaño con GDI+. PERO la
  regeneración de esa ronda no llegó al .ico (por eso Round 7 lo corrige: el
  ico quedó con el estilo viejo).
- **POPOUT DE LLAMADAS**: wry con handler ausente hace SetHandled(true) = NIEGA
  todos los popups. `account_webview_builder` ahora añade `on_new_window` →
  `popup_window()`: crea un WebviewWindow REAL que comparte el `environment`
  del opener (misma sesión/cookies, requisito de WebView2) + UA Chrome +
  client-hints + permisos. Labels `popup-<account>-<n>`.
- **RAIL SIMÉTRICO**: espaciado UNIFORME de 12px — el "+" vive DENTRO del flujo
  como último tile (sube/baja al agregar/eliminar cuentas). Indicador "hay más"
  (chevron con bounce) cuando el rail desborda + scroll con la rueda.
- **RENOMBRAR CUENTAS**: sección "Cuentas" en el drawer y endpoint
  `POST /api/accounts/{id}/rename {label}` (≤40 chars) — verificado.
- **WELCOME con paleta correcta**: reestilizado al gradiente teal con logo
  verde/rayito negro + soporte de tema claro. El splash de arranque en 150ms.
- PowerShell+curl JSON: usar `--data-binary @file` (las comillas simples con
  `{...}` se las come el parser nativo).

## Round 6 (feedback 4: bughunt de tema/z-order/scroll, desinstalación virgen)

- El usuario DESINSTALÓ para versión virgen → el hook NSIS borró
  `%APPDATA%\app.zapzap.desktop` COMPLETO. Luego reinstalado NSIS con `/S`.
- **TEMA MITAD/MITAD (franja blanca + franja oscura)**: el welcome (y el drawer)
  aplicaban el tema UNA vez al crearse; el welcome queda PARQUEADO y jamás
  recarga. FIX: (1) post_prefs → `reload_shell_pages` re-navega rail+welcome al
  instante; (2) `toggle_drawer` CIERRA+RECREA el drawer en cada apertura
  (siempre arriba + estado fresco); (3) el rail ya re-chequeaba prefs cada 10s.
- **DIBUJO DUPLICADO DEL RAIL**: la estructura `#rail{display:contents}` rompía
  el diff. FIX: estructura real `.rail-scroll > #rail + addBtn`; iconos opacidad
  0.85 → 1.
- **Borrado de perfil al eliminar cuenta**: `close_account_webview` ahora
  TAMBIÉN hace remove_dir_all del perfil webview/<id> (best-effort).

## Round 8 (feedback 6: identidad propia + AGPL-3.0 + icono de taskbar en caché)

- **PUBLICADO EN GITHUB (Round 8b)**: gh 2.103.3 instalado por winget; login
  device-flow del usuario (code 3A27-89C8) → auth ok como **ldpf96** (id
  19356038, scopes repo/read:org/gist). Identidad git local establecida y
  TODA la historia reescrita con filter-branch env-filter a
  `ldpf96 <19356038+ldpf96@users.noreply.github.com>`; refs/original y
  reflogs purgados (pack 686 KiB total). Repo creado:
  **https://github.com/ldpf96/zapdude** (público, default branch `main`,
  GitHub lo detecta como **GNU AGPLv3**). Tag `v1.0.0` (nuevo, sobre el
  head reescrito; el tag viejo local estaba en el pre-reescritura y se
  borrió). Release:
  **https://github.com/ldpf96/zapdude/releases/tag/v1.0.0** con el asset
  `Zapdude_1.0.0_x64-setup.exe` (2.81 MiB) y las notas en
  `docs/release-notes-1.0.0.md`. Topics añadidos: whatsapp, tauri, rust,
  windows, whatsapp-web, multi-account, desktop-client.
- Pre-push checks: scan de secretos sobre archivos rastreados CLEAN;
  target/ NUNCA fue commitido (.gitignore ya lo cubría); historial real
  94 archivos (~1.2 MiB loose).

- **IDENTIDAD/ATRIBUCIÓN (decisión del usuario)**: Zapdude es ahora presentado
  como PROYECTO ORIGINAL de **ldpf96** con **AGPL-3.0-or-later** (LICENSE ya
  era el texto AGPL-3.0 ✓). Todo texto "reescritura/fork/obra derivada/crédito
  obligatorio" fue REMOVIDO: README (intro + Licencia), NOTICE.md (reescrito
  completo: proyecto original, inspiración cortesía, sin afiliación
  WhatsApp/Meta), CHANGELOG, Cargo.toml `authors = ["ldpf96"]`, drawer.html
  "Acerca de" (autor ldpf96 + link + inspo ZapZap), index.html title, y
  comentarios de fuente (mod.rs/config.rs/main.rs/window.rs/protocol.rs/
  page_ops.rs/server.rs/events.rs/scripts/*.js/installer-hooks.nsh/
  make_icon.py/architecture.md). Identidad de red renombrada:
  `X-Zapdude-Signature` (antes X-ZapZap-Signature), UA `Zapdude-API/1`,
  flag `window.__zapdudeApiV1` (antes `__zapZapApiV1`; mismo flag en
  window.rs deferred + page_ops.rs + api_page_bridge.js). El identifier
  config `app.zapzap.desktop` NO cambia (preserva el config del usuario).
- **ICONO DE TASKBAR GRIS (caché de Windows)**: el exe YA llevaba el ico
  verde/negro incrustado (PROBADO: extraction con ExtractIconEx del exe
  instalado → 32x32/16x16 verdes con rayo negro; samples 106,233,156 /
  4,35,26 / 20,167,109). Lo que veía el usuario gris era la CACHÉ DE ICONOS.
  REMEDIO aplicado: matar zapdude → borrar `%LOCALAPPDATA%\IconCache.db` +
  `iconcache_*.db` de Explorer → reiniciar explorer.exe → relanzar app →
  **taskbar button VERDE con rayo negro visible y verificado con captura**
  (temp v8-taskbar-zapdude.png). Nota: la app arranca a la bandeja si está
  activo "Iniciar minimizado" (sin botón de taskbar hasta abrir la ventana).
- Commit `6de76a6` (Round 7) y este Round 8 van en el próximo commit.

## Round 7 (feedback 5: icono viejo, cierre del drawer lento, renombrado que borra)

- **ICONO GRIS CON RAYO BLANCO (la queja principal)**: el `icons/icon.ico`
  incrustado en el exe seguía siendo el ESTILO VIEJO (teal oscuro con rayo
  blanco y aro verde) aunque los PNG pequeños ya eran el estilo aprobado —
  una regeneración anterior dejó el ico sin tocar. El exe/taskbar/bandeja/
  título/instalador/acceso directo muestran el ico → por eso el usuario seguía
  viendo el viejo ("dentro se ve bien, fuera se ve feo"). FIX: `icon.ico`
  reconstruido BYTE A BYTE desde los PNG aprobados (16/24/32/48/64/128/256
  PNG-comprimidos, 7 frames); `icon.png` (512) + `Square*Logo.png` +
  `StoreLogo.png` re-renderizados con la receta GDI+ del generador original
  (squircle r=0.23, degradado vertical #34e877→#0f9e6c, brillo diagonal,
  rayito #04231a al 60% — `scripts/regen-icons.ps1`; auditor
  `tools/dump-ico.ps1`). VERIFICADO leyendo el HICON REAL de la ventana viva
  (WM_GETICON + GetIconInfo + GetObject): 32x32 con rayito negro (samples:
  99,222,154 / 4,35,26). NOTA: los assets android/ios (`ic_launcher*`) SIGUEN
  viejos (no se usan en Windows; regenerar en el CI si algún día se apunta a
  móvil), e `icon.icns` mac pendiente igual.
- **CIERRE DEL DRAWER LENTO ("se tarda un poco la X")**: `toggle_drawer` al
  cerrar SOLO cambiaba el flag; el webview se estacionaba cuando corría el
  watchdog de geometría (cada 2 s) porque `set_focus` no dispara `Focused` con
  la ventana ya enfocada. FIX: `relayout` directo dentro de `toggle_drawer`
  (también cubre apertura). MEDIDO: estacionar a x=-24000 en **1 ms** (poll de
  rects del child WRY_WEBVIEW contra POST /api/ui/drawer; antes: hasta 2 s).
  La apertura sigue con destrucción+recreación (z-order encima; el usuario la
  aprueba).
- **RENOMBRAR CUENTAS BORRABA LO ESCRITO**: la sección Cuentas del drawer
  reconstruía `#accountsList` con innerHTML="" cada 5 s → reset del valor en
  el input + robo de foco justo mientras escribías (imposible terminar el
  nombre — "pareciera que está constantemente actualizándose"). FIX en
  `ui/drawer.html`: filas persistentes por id (`accountRows` Map); refresh
  incremental que (1) elimina filas de cuentas ya inexistentes, (2) AGREGA
  cuentas nuevas, (3) actualiza el label solo del input NO enfocado y NO
  "dirty" (dataset.dirty lo marca el listener 'input'), (4) el mensaje de
  "todavía no hay cuentas" ahora es persistente (`#noAccounts`). Verificado E2E en browser real contra el server
  vivo (http://127.0.0.1:29977/drawer): escribir "Nombre QA de Prueba" → pasó
  >1 tick de refresh sin perder texto ni foco; rename EXTERNO simultáneo NO lo
  pisó; clic en el check → toast "Cuenta renombrada" + server persistió; tras
  blur, la fila fusiona el label del servidor (account-6 restaurada a
  "Cuenta 6").
- Instalación en silencio del NSIS nuevo `/S` (exe 2,943,632 bytes; instalado
  11,274,752 @ 2026-10-28 14:56) + `ie4uinit -show`. Tests 14/14.
- Lección operativa: `Start-Process` con ruta RELATIVA falla ("no se puede
  encontrar el archivo") aunque el CWD parezca correcto; usar la ruta
  absoluta del instalador.

## Estado de verificación (con evidencia real)

- ARRANQUE VIRGEN: sin config → auto-crea "Cuenta 1" (account-1) con QR
  (dot naranja) — nunca "default".
- TEMA: dark→light→dark uniforme (el usuario lo spameó y "funciona perfecto").
- SWITCH de cuentas: instantáneo por parking position-only (<350ms medidos).
- "+" en flujo bajo la última cuenta; crea y abre su QR al instante; scroll
  del rail con rueda + indicador "hay más".
- DELETE por clic-derecho-armado con X roja visible + perfil borrado.
- BANDEJA: "Mostrar <cuenta>" restaura la ventana; menú dinámico (set_menu);
  clic izquierdo en el icono trae la ventana; "Ocultar ventanas" oculta todo.
- AUTOSTART: POST prefs autostart escribe/borra HKCU Run "Zapdude" ✓.
- LANDING del QR 100% limpio (banner de descarga eliminado por completo).
- Guard de instancia única: 2ª instancia sale sola.
- QUIT desde API: proceso termina limpio.
- POP-OUT de llamadas: handler on_new_window instalado — PENDIENTE prueba
  manual del usuario.
- ICONO (Round 7): ico re-ensamblado desde PNGs aprobados; verificado por
  dump de frames + HICON de la ventana viva.
- RENAME (Round 7): E2E real contra el server (browser driver).
- DRAWER CLOSE (Round 7): 1 ms medido.

## PENDIENTE (bughunt activo del usuario — atender feedback con capturas)

- **Prueba EN VIVO del usuario** del binario nuevo: icono verde en
  taskbar/bandeja (la caché de iconos puede pedir reiniciar el Explorador),
  pop-out de llamadas, QR escaneado, bandeja completa.
- **Rediseñar los eventos push** (notificaciones → API/webhook) de forma
  segura: hoy la API pasiva funciona (status/chats/send/open-chat) pero SIN
  eventos espontáneos; idealmente CDP/WebMessageReceived o polling del host.
- Iconos del rail estilo original (llamadas, favoritos, canales, etc.) y
  tabs adicionales (Canales, favoritos, mute, descargas, donar) — decidir
  alcance cuando el usuario lo pida.
- Regenerar los assets android/ios (`ic_launcher*`) e `icon.icns` (mac) al
  estilo nuevo — en el CI al publicar.
- Workflow CI (GitHub Actions) para compilar Windows/macOS/Linux gratis en
  releases públicos (el usuario lo aprobó como plan para Mac). Regenerar
  icon.icns para mac en el CI.
- Publicar en GitHub: ✓ HECHO (Round 8b): https://github.com/ldpf96/zapdude
  + release v1.0.0 con el instalador. PENDIENTE: workflow CI (GitHub
  Actions) para builds Windows/macOS/Linux y regeneración de icon.icns; y
  actualizar el README badge del release si cambian versiones futuras.

## Comandos útiles

```powershell
# Build + tests + instalador
cd C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
cargo test
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

- PowerShell: cargo/tauri escriben a stderr → ruido `NativeCommandError`
  cosmético; capturar con `$o = cmd 2>&1; $LASTEXITCODE`.
- **El mismo shell de PowerShell PERSISTE env vars entre comandos** — la env
  var de debug `ZAPDUKE_WEBVIEW_OPTS` contaminó varias corridas; limpiarla SIEMPRE.
- `curl.exe` (no alias de PS). Para POST con body JSON usar archivos o
  Invoke-RestMethod.
- `Start-Process` con ruta relativa FALLA aunque el CWD sea el correcto:
  SIEMPRE ruta absoluta.
- Los .ps1 en UTF-8: NUNCA usar em-dashes u otros no-ASCII dentro de strings
  de PowerShell (parsing se rompe); español plano en comentarios.
- git: `C:\Program Files\Git\bin\git.exe` (PATH puede no estar refrescado).
- Usuario habla español; responder en español.
