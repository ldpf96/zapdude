# ESTADO DE LA SESIÃ“N â€” Zapdude (Rust)

> Documento de continuidad para agentes/compaction. Ãšltima actualizaciÃ³n: 2026-10-28 (tarde, v6).
>
> **SIGUIENTE PASO INMEDIATO**: el usuario COMPACTÃ“ y estÃ¡ PROBANDO en vivo la
> versiÃ³n nueva instalada (QR, scroll del rail, tema, bandeja, pop-out de
> llamadas, borrado con X). Atender su feedback con capturas. Prioridad de
> verificaciÃ³n pendiente: llamadas/voide pop-out y QR real.
>
> La app se llama **Zapdude** (Z mayÃºscula, resto minÃºscula â€” aclaraciÃ³n
> EXPLÃCITA del usuario: "Debe ser Zapdude, no ZapDude"; NO usar "ZapDude").
> Mantener crÃ©dito a Rafael Tosta por ZapZap. Todo verificado con capturas
> de ventana reales (PrintWindow + PW_RENDERFULLCONTENT). Instalador actual:
> `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\target\release\bundle\nsis\Zapdude_1.0.0_x64-setup.exe`
> (~2.81 MiB).

## Estado actual del usuario (importante, v6)

- **El usuario DESINSTALÃ“ para versiÃ³n virgen** â†’ el hook NSIS borrÃ³
  `%APPDATA%\app.zapzap.desktop` COMPLETO (config + perfiles â€” incluida la
  sesiÃ³n "default" que tenÃ­a logueada; limpio a propÃ³sito, Ã©l lo pidiÃ³).
- DespuÃ©s re-instalÃ© en silencio (`/S`) el NSIS nuevo: binario
  `C:\Users\PC\AppData\Local\Zapdude\zapdude.exe` (release, 11.2 MB,
  2026-10-28 14:01). Title bar "Zapdude" + icono verde/rayito negro âœ“.
- La app estÃ¡ CORRIENDO con el binario nuevo: **"Cuenta 1" (account-1)
  auto-creada, dot naranja = QR esperando**, tema oscuro, autostart OFF,
  API desactivada. Captura final: `C:\temp\n1-virgen-final.png`.
- Theme toggle verificado uniforme (light m2 / dark m3, sin franjas mixtas).
- IE 4 icon cache refrescado (ie4uinit -show) para la taskbar.

## QuÃ© es este proyecto

Reescritura completa en **Rust** del cliente **ZapZap** (original de **Rafael Tosta**,
Python/GTK, https://github.com/rafatosta/zapzap, GPL-3.0). Este fork se distribuye
como **AGPL-3.0** (obra derivada, Â§13) con crÃ©dito obligatorio a Rafael Tosta
(see `NOTICE.md`).

Requisitos del usuario (en orden de importancia):
1. **Multi-cuenta vital**: todas las cuentas de WhatsApp en UNA app, notificaciones
   unificadas. Experiencia EXACTAMENTE como ZapZap original: rail lateral con
   avatares, click en "+" agrega cuenta SIN FORMULARIOS â†’ QR â†’ escanear â†’ entrar.
   El usuario dijo: "hazlo en un principio igual [al original] y si quieres a
   partir de ahÃ­ lo mejoras".
2. **Interfaz humana** profesional â€” "mÃ¡s premium, mÃ¡s profesional" â€” que la gente
   quiera usarlo.
3. **API local** en loopback para humanos y **agentes de IA** (REST + SSE + webhook HMAC).
4. **Llamadas/vÃ­deo/pantalla** funcionando (identidad Chrome 156 en JS).
5. **Sin terminal** al abrir, **instalador Windows NSIS**, desinstalaciÃ³n completa.
6. "Que no se rompa nunca": errores degradan, no crashean.

## Ubicaciones

- **Proyecto**: `C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs`
- **Instalador**: `target\release\bundle\nsis\ZapDude_1.0.0_x64-setup.exe` (~2.84 MiB)
- **Config runtime**: `%APPDATA%\app.zapzap.desktop\config.json`
  (camelCase, tolerante a BOM: `api{enabled,port,apiKey,exposeNetwork,webhookUrl,webhookSecret}` + `accounts[{id,label,enabled}]`)
- **Perfiles WebView2**: `%APPDATA%\app.zapzap.desktop\webview\<account-id>`
- **ZapZap original (referencia, NO tocar)**: `C:\Users\PC\Desktop\Projects\Zapdude\zapzap-main`
- **tauri CLI prebuilt**: `zapdude-rs\tools\tauri.exe` (usar: `.\tools\tauri.exe build --bundles nsis`)

## Arquitectura actual (v3 â€” drawer + position-only parking)

**UNA ventana "main"** (Tauri 2, feature `unstable`, multi-webview):
- Webview `rail` (76px lÃ³gico, izquierda): `rail.html` servido POR el API server.
  Solo: **logo ZapDude (rayito SVG) arriba** â†’ **avatares** (squircles 44px con el
  rayito ZapDude â€” SIN letras/nÃºmeros; activa = anillo verde + glow; dot de estado:
  verde=conectada, naranja=espera QR, gris=cargando) â†’ **botÃ³n "+"** abajo.
  Sin footer, sin tuerquita, sin â“˜, sin versiÃ³n, sin divisoria (todo eso vive en
  el drawer). Poll cada 2.5s con **diff incremental de DOM** (renderRail por
  dataset.id â€” sin flicker).
  - **Logo = toggle drawer de opciones** (POST /api/ui/drawer).
  - Click en avatar = abrir/cambiar a esa cuenta.
  - **Eliminar cuenta**: clic derecho sobre el avatar lo "arma" (rojo Ã—, pulso) â†’
    clic de nuevo elimina; Esc o clic en otro lo desarma (SIN auto-desarme de
    tiempo â€” un timer de 4s se comÃ­a los clicks lentos y "no dejaba borrar").
- Webview `welcome`: backdrop tÃ©cnico (NUNCA es destino de ningÃºn flujo).
- **Webview `drawer`** (LAZY, se crea al primer toggle para quedar ENCIMA de las
  cuentas): `drawer.html` servido por el server â€” panel de 380px sobre el contenido
  con secciones separadas: **ConfiguraciÃ³n API** (toggle + badge Activa/Desactivada,
  puerto, clave, webhook, guardar), **Agente de IA** (endpoints), **Acerca de**
  (ZapDude, AGPL-3.0, crÃ©dito Rafael Tosta). Cierra con X o Esc. La ventana
  "settings" separada quedÃ³ en el cÃ³digo PERO la UI ya no la usa.
- Webviews `account-<id>`: WhatsApp Web (`https://web.whatsapp.com`), cada uno con:
  - `data_directory` aislado (perfil propio por cuenta)
  - UA Chrome 156.0.8078.49 + `client_hints_fix.js` (identidad JS para llamadas) â€”
    inyectado en document-start (startup script, pequeÃ±o)
  - `api_page_bridge.js` â€” **INYECTADO DIFERIDO** (ver "LecciÃ³n crÃ­tica")
  - `hide_download_banners.js` â€” **REESCRITO SEGURO** (ver "LecciÃ³n crÃ­tica")
  - Permisos auto-Allow: Microphone, Camera, DisplayCapture, Notifications, ClipboardRead, PointerLock
- **VISIBILIDAD (modelo parked offscreen, position-only)**: el webview activo va
  en `(RAIL_WIDTH, 0)` a tamaÃ±o completo; los inactivos van a **x=-24000
  (PARKED_X) TAMBIÃ‰N A TAMAÃ‘O COMPLETO**. El switch es SOLO movimiento de
  posiciÃ³n â†’ WhatsApp NO re-renderiza â†’ cambio instantÃ¡neo (<350ms medidos).
  (El modelo anterior 1Ã—1 forzaba re-render completo de WhatsApp: 2-3s por switch.)
- **Arranque (orden crÃ­tico en main.rs setup)**:
  1. `enforce_single_instance()` (mutex `Local\app.zapzap.desktop.single`; 2Âª
     instancia sale inmediatamente â€” evita peleas de puerto/config)
  2. `start_loopback_server()` â€” **ANTES de crear webviews** (antes la UI cargaba
     antes de que el server existiera â†’ error Edge "can't reach this page" al abrir)
  3. `create_shell` (rail + welcome)
  4. `ensure_account_webview` por cuenta (nacidos a TAMAÃ‘O COMPLETO y luego
     parqueados; un webview nacido 1Ã—1 u oculto queda gris para siempre)
  5. `auto_open_first_account` â€” abre la primera cuenta directo (sin welcome
     interstitial; crea `Cuenta 1` si no hay ninguna â€” comportamiento ZapZap original)
  6. tray
- Carrera "already exists": `window.add_child` se despacha async al hilo principal;
  si `add_child` responde "already exists" se trata como Ã©xito (no fallar el open).
- Ventana "settings" separada (`settings.html?tab=api|agent|about`), servida por el server.
- Cerrar ventana main = ocultar (tray mantiene la app viva).
- Tray: "Mostrar <cuenta>" por cuenta, "Ocultar ventanas", "Salir de ZapDude".
- API server: bind con RETRY (40 Ã— 250ms) â€” el restart tras `POST /api/config`
  fallaba con error 10048 (puerto aÃºn ocupado por el listener viejo).
- `list_accounts` responde AL INSTANTE con cachÃ© de session_state (TTL 2.5s,
  refresco en background con tokio::spawn) â€” el rail ya no espera 1.2s de snapshots.

**Clave del diseÃ±o**: el **API server sirve la propia UI** (`GET /` â†’ rail.html,
`GET /welcome`, `GET /settings`, `GET /drawer` â€” embebidos con include_str!). Todas
las pÃ¡ginas hacen `fetch` **mismo-origen** â†’ imposible fallar por CORS/IPC/file://.
NO se usa Tauri IPC para la UI (los comandos de `app_commands.rs` siguen
registrados pero la UI usa HTTP).

## âš ï¸ LECCIÃ“N CRÃTICA â€” la "pÃ¡gina gris" (2 causas encadenadas)

SÃ­ntoma: rail/welcome perfectos; al abrir una cuenta, el Ã¡rea de contenido queda
en gris plano, PERO el DOM de WhatsApp estÃ¡ vivo (los snapshots del bridge veÃ­an
el QR â€” `session_state: scan_qr`). Verificado con capturas PrintWindow
(PW_RENDERFULLCONTENT=2) en `C:\temp\*.png`.

**Causa raÃ­z 1 â€” webviews creados ocultos o nacidos enano quedan grises para
siempre.** WebView2 (child webviews en wry 0.57) solo arranca su pipeline de
composiciÃ³n si nace a tamaÃ±o real en el slot de contenido. Fix: nacer a tamaÃ±o
completo + parqueo por POSICIÃ“N (x=-24000, tamaÃ±o completo â€” ver Arquitectura).
NOTA: al inicio se creyÃ³ que el parking offscreen tambiÃ©n causaba gris y se pasÃ³
por 1Ã—1; despuÃ©s de arreglar la causa raÃ­z 2 se comprobÃ³ que el parking offscreen
funciona perfecto y es el que da switches instantÃ¡neos.

**Causa raÃ­z 2 â€” `hide_download_banners.js` (versiÃ³n vieja) borraba la pÃ¡gina.**
Su barrido genÃ©rico marcaba `__zapdudeChecked` por nodo y ocultaba cualquier
contenedor cuyo `textContent` coincidiera con el patrÃ³n de promo; los
contenedores grandes del landing coinciden (su texto contiene "Descarga WhatsApp
para Windows") â†’ TODA la pÃ¡gina quedaba `display:none` â†’ fondo oscuro plano.
CorrelaciÃ³n engaÃ±osa: el bridge solo parecÃ­a "vivo" en las corridas grises
(por el orden en que cada script se instalaba/truncaba). Fix: reescritura
"smallest matching node only" + colapso controlado de ancestros (mÃ¡x 6 niveles,
solo si TODOS los hijos quedan display:none computado, contenedores de â‰¤4 hijos).

**Hallazgos colaterales del debug (empÃ­ricos, no tocar sin re-testear)**:
- Pasar `additional_browser_args` custom en child webviews + UA Chrome â‡’ gris
  (probado: full set, solo ms*, solo bg-flags, wry-default explÃ­cito â€” TODOS grises
  con UA; sin args custom â‡’ pinta). Por eso NO se pasan args custom en producciÃ³n.
- `BROWSER_ARGS` viejo usaba `\` de continuaciÃ³n de Rust que pega tokens
  ("...Protection--disable-...") â€” malformado ademÃ¡s.
- Un solo initialization script combinado de ~23 KB tambiÃ©n se asociÃ³ al gris;
  los scripts ahora van separados: startup = client_hints (5.5KB); bridge+promos
  van DIFERIDOS vÃ­a eval idempotente desde el poller de beacons
  (`DEFER_INSTALL_EVERY_TICKS=100` â‰ˆ cada 3s, chequeo `!window.__zapZapApiV1`).
- El transporte de eventos push (hook `window.Notification` + observer chat_list)
  quedÃ³ DESHABILITADO en el bridge (era parte del enredado gris). La API pasiva
  (status/chats/send/open-chat) funciona. Eventos push: pendiente de rediseÃ±o seguro.

## Debug harness (reutilizable)

- Captura de ventana con contenido WebView2: PrintWindow + `PW_RENDERFULLCONTENT=2`
  (los webviews son DirectComposition; sin ese flag sale vacÃ­o). Scripts en el
  historial; guardar en `C:\temp\*.png` y leerlas con visiÃ³n.
- BisecciÃ³n por env var `ZAPDUKE_WEBVIEW_OPTS` (window.rs `has()`):
  `ua|scripts|data|args|perms|scripts1|scripts2|scripts3|argsbg|argsms|argsry` â€”
  dejar el harness; es barato y ya salvÃ³ la sesiÃ³n dos veces.

## Historial de fixes importantes (por quÃ© pasaron)

1. axum 0.8 `.nest("/")` pÃ¡nico "Nesting at the root" â†’ rutas planas.
2. WebView2 `ExecuteScript` llega **doble-codificado** cuando el valor es String â†’
   `evaluate()` desenrolla una vez (sin esto todo caÃ­a a defaults silenciosos).
3. `session_state` honesto a 3 estados: detecting/scan_qr/logged_in.
4. UI anterior usaba fetchâ†’server no corriendo + CORS file:// â†’ "Failed to fetch".
   Fix: server siempre corre + UI servida por Ã©l (mismo origen).
5. UI anterior usaba `window.__TAURI__` IPC que fallaba silencioso â†’ botones muertos.
6. `windows_subsystem = "windows"` en main.rs (sin consola).
7. Rate limit 60/min, body 1MB, ring 200 eventos, SSE keep-alive 15s.
8. 14 tests unitarios (`cargo test`) SIEMPRE en verde antes de entregar.
9. (HOY) Server ANTES de shell; auto-open primera cuenta; nacer a tamaÃ±o completo;
   parqueo offscreen position-only (switch instantÃ¡neo); "already exists" no fatal;
   hide-banners reescrito smallest-node + zonas protegidas; eventos push del bridge
   desactivados; bind con retry; single-instance mutex; drawer de opciones en la
   misma ventana; cachÃ© de session_state; deleteAppDataOnUninstall no soportado
   por el tauri-build local â†’ NSIS hook `installerHooks: ./windows/installer-hooks.nsh`
   (RMDir /r del data dir al desinstalar).

## Commits (en orden)

- `d08eb65` feat: initial Rust rewrite (Tauri 2) â€” multi-account, tray, API, calls identity
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

## Round 3 (feedback: logo-menus, velocidad, parpadeo, footer)

- **Drawer de opciones en la MISMA ventana** (ui/drawer.html, label `drawer`,
  DRAWER_WIDTH=380): el logo ZapDude del rail lo abre/cierra
  (POST /api/ui/drawer â†’ window::toggle_drawer, creado LAZY la primera vez para
  que quede ENCIMA de las cuentas). Secciones separadas: ConfiguraciÃ³n API
  (form + badge Activa/Desactivada + guardar), Agente de IA (endpoints), Acerca de
  (ZapDude + AGPL + crÃ©dito Rafael Tosta). Cierra con X o Esc.
- **Rail sin footer**: solo logo â†’ avatares â†’ "+". Sin tuerquita, sin â“˜, sin
  versiÃ³n, sin la lÃ­nea divisoria pegada al "+".
- **Switch INSTANTÃNEO**: las cuentas inactivas van parqueadas a TAMAÃ‘O COMPLETO
  en x=-24000 (PARKED_X) y el switch es SOLO movimiento de posiciÃ³n â€” WhatsApp no
  re-renderiza (con 1Ã—1 re-renderizaba completo = 2-3s). Medido: pintado completo
  a 350ms. El viejo "gray al parkear offscreen" era el script de promos (ya
  reescrito) â€” el parking offscreen funciona bien con los scripts arreglados.
- **Rail sin parpadeo**: renderRail() hace diff incremental (por dataset.id,
  insertBefore en orden) en vez de innerHTML="" por cada poll (2.5s) o add.
- open_account_webview cierra el drawer (el QR/chat toma todo el Ã¡rea).
- reload_shell_ui re-apunta tambiÃ©n el drawer (`/drawer`).
- Config de la ventana "settings" y su endpoint /api/ui/settings quedan en el
  cÃ³digo pero la UI ya no los usa (todo vive en el drawer).

## Round 4 (feedback 2: pantalla completa, lazy load, X visible, "Zapdude", publicar)

- **Opciones a PANTALLA COMPLETA**: el drawer (logo del rail) ahora cubre TODO
  el Ã¡rea de contenido (de DRAWER_WIDTH=380 a content.width), columna centrada
  max-width 780px. El rail sigue visible para que el logo siga haciendo toggle.
  DRAWER_WIDTH eliminado. Verificado con captura (f2-drawer-fullscreen).
- **Carga diferida de cuentas** (velocidad con muchas cuentas): main.rs YA NO
  crea un webview por cuenta al arranque â€” solo `auto_open_first_account`;
  las demÃ¡s cuentas crean su webview LAZY en el primer open
  (`open_account_webview` â†’ `ensure_account_webview` ya lo hace). Arranque
  instantÃ¡neo con N cuentas. Trade-off: las cuentas de fondo no dan
  notificaciones hasta abrirse una vez por sesiÃ³n (documentado en README).
- **X VISIBLE al armar con clic derecho**: rail.html aÃ±ade symbol `#i-x` y
  renderRail hace `use.setAttribute('href', armed ? '#i-x' : '#i-bolt')`.
  Verificado con captura (h2-rail-crop: X roja clara sobre tile rojizo).
- **Divisorias curvas estÃ©ticas** en el rail: `.divider.below` (bajo el logo)
  y `.divider.above` (sobre el "+") â€” hairline verde con gradiente y borde
  curvo (border-radius porcentual). Verificado en crop del rail.
- **MenÃº contextual del navegador BLOQUEADO** en la UI: `contextmenu â†’
  preventDefault` en rail/drawer/welcome (el clic derecho sobre el logo ya no
  muestra el menÃº de pÃ¡gina web). Los webviews de WhatsApp conservan SU menÃº.
- **Borrar la Ãºltima cuenta NO auto-crea una vacÃ­a**: nuevo
  `open_first_remaining_or_welcome` (window.rs) â€” abre la primera cuenta
  restante o muestra el welcome (que YA tiene botÃ³n "+ Agregar cuenta").
  `close_account_webview` y `hide_account_content` lo usan. El arranque
  (`auto_open_first_account`) SÃ crea "Cuenta 1" si no hay ninguna (flujo
  ZapZap original intacto). Verificado: DELETE Ãºltima â†’ `accounts: []` +
  welcome con CTA (captura h4). Luego se recreÃ³ account-1 para dejar la app
  como estaba.
- **Renombre a "Zapdude"** (Z mayÃºscula, resto minÃºscula â€” aclaraciÃ³n final
  del usuario): productName, tÃ­tulo de ventana, tray, healthz service,
  status name, textos UI, README, NOTICE. El binario sigue `zapdude.exe`
  (cargo name) y el identifier `app.zapzap.desktop` NO cambia.
- **Limpieza de cÃ³digo muerto** (repo listo para publicar): eliminados
  `app_commands.rs` (mÃ³dulo + invoke_handler completo â€” la UI es 100% HTTP),
  ventana "settings" (`create_settings_window`), rutas `/settings` y
  `/api/ui/settings`, `ui/settings.html`, `withGlobalTauri: false`.
- **FIX CRÃTICO del hider de banners (banner del QR)**: causa raÃ­z TRIPLE â€”
  (1) el instalador diferido envolvÃ­a bridge+hider en UN SOLO eval con la
  marca `__zapZapApiV1`: si el bridge ya estaba marcado, el hider no se
  instalaba nunca. Ahora `deferred_scripts()` devuelve pares (flag, script)
  y page_ops instala cada uno con SU flag y SU try/catch. (2) El botÃ³n del
  promo card es un BUTTON cuyo textContent es "Downloadic-download" (el svg
  interno trae un title "ic-download" que se concatena al texto) â†’ nuevo
  `cleanText()` que elimina tokens `ic-*` antes de los patrones exactos.
  (3) El card completo se oculta con `hideLandingPromoCard()`: busca el
  botÃ³n "Download"/"Descargar" (texto limpio) y sube al ancestro MÃS CHICO
  que contenga texto promo con presupuesto de 400 chars â€” el card del QR
  (sin texto promo) jamÃ¡s matchea. Guardias: solo corre sin `#pane-side`/
  `#main` VISIBLES (WhatsApp pre-renderiza #main oculto en el landing â€” la
  existencia no basta; usar getClientRects) y los botones ya ocultos SÃ se
  siguen (son la pista hacia el card â€” el barrido genÃ©rico los oculta
  primero). DiagnÃ³stico vÃ­a beacon temporal + /api/events (cadena de
  ancestros del botÃ³n). Verificado con captura g8: landing 100% limpio,
  QR centrado, card del QR intacto.
- NOTA de operaciÃ³n: una ediciÃ³n del config con PowerShell `-replace` tocÃ³
  TODOS los `"enabled":true` (tambiÃ©n el de la cuenta) â€” restaurado a mano;
  cuidado con replace global en JSON.
- `withGlobalTauri: false` (no se usa IPC).

## Round 5 (feedback 3: tema, anclaje, bandeja, autostart, icono, popouts, rail simÃ©trico)

- **DIAGNÃ“STICO CLAVE**: el usuario estaba corriendo la app VIEJA instalada
  (`C:\Users\PC\AppData\Local\ZapDude\zapdude.exe`, tÃ­tulo "ZapDude", sin las
  X ni nada de lo nuevo) â€” de ahÃ­ sus quejas de "no sale la X" y colores
  viejos. TambiÃ©n tenÃ­a el ZapZap original 7.5.1 corriendo desde Descargas.
  Al final de la ronda se instala el NSIS nuevo en silencio.
- **Reset del config a "default"** (account "default" zombi reapareciÃ³): fue
  una CARRERA entre el binario viejo instalado y el nuevo â€” el viejo leyÃ³ un
  config.json a medio escribir â†’ parse fail â†’ `unwrap_or_default` â†’
  persistiÃ³ el default. FIX: `SettingsState::load` ahora reintenta la lectura
  (4 intentos Ã— 60ms) ante JSON invÃ¡lido; la carrera desaparece cuando solo
  corre el binario nuevo (mismo mutex de instancia Ãºnica).
- **TEMA CLARO/OSCURO** (pedido explÃ­cito): nueva secciÃ³n Preferencias en el
  drawer con switch "Tema claro". Config: `AppPrefs { theme, autostart,
  startMinimized }` (camelCase, serde default â€” compat con configs viejos).
  Endpoints GET/POST /api/prefs (management, loopback). rail/drawer/welcome
  leen prefs y aplican `data-theme` en `<html>` con overrides de variables
  CSS (`:root[data-theme="light"]`). El rail re-chequea el tema cada 10s; el
  drawer aplica al instante. El tema NO toca la pÃ¡gina de WhatsApp (cada
  cuenta usa el tema que tenga en WhatsApp).
- **NAVEGADOR ANCLADO A LAS 4 ESQUINAS** (bug: "el chat desapareciÃ³, se
  achicÃ³ de derecha a izquierda"): relayout ahora tambiÃ©n corre en
  `ScaleFactorChanged` y `Focused` (ademÃ¡s de Resized), MÃS un watchdog de
  geometrÃ­a cada 2s (thread "zapdude-geometry") que re-alinea mientras la
  ventana no estÃ© minimizada. El drift de layout ya no sobrevive.
- **BANDEJA ARREGLADA** ("Mostrar cuenta nada de eso funciona"): (1) al hacer
  clic en "Mostrar <cuenta>" la ventana principal se muestra/des-minimiza/
  enfoca ANTES de abrir el webview (mostrar un webview dentro de una ventana
  oculta no hace nada visible â€” ese era el bug); (2) `refresh_tray` reconstruye
  el menÃº vÃ­a `tray_by_id`+`set_menu` cuando se agrega/elimina/renomina una
  cuenta (antes quedaba viejo); (3) clic IZQUIERDO en el icono de la bandeja
  trae la ventana (`on_tray_icon_event`, menÃº con right-click);
  (4) "Ocultar ventanas" ahora oculta TODAS las ventanas de verdad.
- **AUTOSTART + INICIO MINIMIZADO** (pedido explÃ­cito): Preferencias â†’
  "Iniciar con Windows" (HKCU Run key vÃ­a crate winreg, sin admin) y
  "Iniciar minimizado" (arranca sin ventana, vive en la bandeja). Verificado:
  POST prefs autostart:true escribe `HKCU\...\Run\Zapdude = <exe>`; false lo
  borra (NotFound = ok).
- **ICONO DE LA APP** (pedido: "el verde con rayito negro estaba bien bonito,
  Ãºsalo como logo del software"): generados icons/*.png (16..256) + icon.ico
  multi-tamaÃ±o con GDI+ (scripts temporales en Temp/opencode): squircle verde
  con degradado #34e877â†’#0f9e6c, highlight diagonal sutil y rayito negro
  #04231a (path del rail escalado 62%). La bandeja y el exe lo usan.
- **POPOUT DE LLAMADAS** ("modifica tu navegador para que permita popups"):
  wry con handler ausente hace `SetHandled(true)` = NIEGA todos los popups.
  `account_webview_builder` ahora aÃ±ade `on_new_window` â†’ `popup_window()`:
  crea un WebviewWindow REAL (movible a cualquier lado) que comparte el
  `environment` del opener (ICoreWebView2Environment â€” misma sesiÃ³n/cookies,
  requisito de WebView2) + UA Chrome + client-hints + permisos; tamaÃ±o del
  `features.size()` (default 480Ã—640). Labels `popup-<account>-<n>`.
- **RAIL SIMÃ‰TRICO**: espaciado UNIFORME de 12px (logoâ†’divisoriaâ†’cuentasâ†’"+")
  â€” antes el gap logoâ†’cuenta (18px padding) era mayor que cuentaâ†’cuenta
  (10px), "rancio". El "+" ya NO estÃ¡ clavado abajo: vive DENTRO del flujo
  como Ãºltimo tile (sube/baja al agregar/eliminar cuentas) â€” como pidiÃ³ el
  usuario. Indicador "hay mÃ¡s" (chevron con bounce) cuando el rail desborda +
  scroll con la rueda (mask fade ya existÃ­a).
- **RENOMBRAR CUENTAS** (pedido: "deberÃ­as poder ponerle los nombres"): nueva
  secciÃ³n "Cuentas" en el drawer (inputs + botÃ³n guardar por fila, refresco
  cada 5s) y endpoint `POST /api/accounts/{id}/rename {label}` (â‰¤40 chars) â€”
  verificado (Mi Personal â†” Cuenta 1). Tray y rail muestran el label nuevo.
- **WELCOME con paleta correcta** (el usuario odiaba la pantalla azul-verde
  vieja del arranque): reestilizado al gradiente teal actual con logo verde/
  rayito negro + soporte de tema claro. El splash de arranque quedÃ³ en 150ms.
- PowerShell+curl JSON: usar `--data-binary @file` o `{\"..\"}` escapado (las
  comillas simples con `{...}` se las come el parser nativo).

## Round 6 (feedback 4: bughunt de tema/z-order/scroll, desinstalaciÃ³n virgen)

- **El usuario DESINSTALÃ“ para tener versiÃ³n virgen** â†’ el hook NSIS borrÃ³
  `%APPDATA%\app.zapzap.desktop` COMPLETO (config.json + perfiles webview â€”
  el comportamiento pedido). Estado actual: SIN config; el prÃ³ximo arranque
  auto-crea "Cuenta 1" con QR. Ruta absoluta del instalado:
  `C:\Users\PC\AppData\Local\Zapdude\zapdude.exe` (pedido: dejar la ruta
  visible).
- **TEMA MITAD/MITAD (franja blanca + franja oscura)**: causas â€” el welcome
  (y el drawer) aplicaban el tema UNA vez al crearse; el welcome queda
  PARQUEADO y jamÃ¡s recarga â†’ al mostrarse aparece con el tema viejo; el
  drawer si se creaba antes que las cuentas quedaba debajo (z-order) y
  encima con estado viejo. FIX: (1) post_prefs â†’ `reload_shell_pages` re-
  navega rail+welcome al instante (uniformidad); (2) `toggle_drawer` ahora
  CIERRA+RECREA el drawer en cada apertura (siempre arriba + estado fresco);
  (3) el rail ya re-chequeaba prefs cada 10s. Editar config.json a mano con
  ConvertTo-Json -Depth 5 (Â¡el hash serializer aplana arrays!).
- **DIBUJO DUPLICADO DEL RAIL (solo se veÃ­a 1 cuenta con 9 en config)**: la
  estructura `#rail{display:contents}` rompÃ­a el diff (los tiles no eran
  hijos directos del contenedor de scroll). FIX: estructura real â€”
  `.rail-scroll > #rail (flex column gap 12) + addBtn`, gaps uniformes de
  12px en ambos niveles; opacidad de iconos 0.85 â†’ 1 (mÃ¡s contraste).
- **Borrado de perfil al eliminar cuenta**: `close_account_webview` ahora
  TAMBIÃ‰N hace remove_dir_all del perfil webview/<id> (best-effort) â€” sin
  sesiones huÃ©rfanas en disco (el README ya lo prometÃ­a).

## Round 2 (feedback del usuario tras probar: renombrado + premium + delete)

- **Nombre**: ZapDude en TODAS partes (ventana "ZapDude", "ConfiguraciÃ³n â€” ZapDude",
  tray "Salir de ZapDude", `productName` â†’ instalador `ZapDude_1.0.0_x64-setup.exe`).
  identifier/`app.zapzap.desktop` NO cambia (preserva config).
- **Rail premium** (ui/rail.html): SVG symbol library (bolt/plus/gear/info â€” cero
  emojis), logo squircle con glow que **abre la cuenta activa** (antes no hacÃ­a
  nada), TODOS los avatares muestran el rayito (adiÃ³s letras/nÃºmeros D/2), anillo
  verde + glow en la activa, dot de estado, tooltips glass, fade masks en el scroll.
- **Delete flow**: sin auto-desarme (un timer de 4s se comÃ­a clicks lentos â€” por eso
  "no deja borrar la primera"); right-click arma, click de nuevo elimina, Esc desarma.
  `sanitize_accounts` permite lista VACÃA (adiÃ³s default zombi). Al eliminar la
  cuenta en pantalla â†’ `close_account_webview` â†’ `auto_open_first_account` (abre la
  primera restante o CREA "Cuenta 1" con QR). El welcome NUNCA es destino.
- **Velocidad**: `list_accounts` responde AL INSTANTE con cachÃ© de session_state
  (TTL 2.5s, refresco background con tokio::spawn) â€” el rail ya no espera 1.2s de
  snapshots. Switch medido: <500ms con render completo (capturas).
- **Promo card eliminado completo** del panel de chat vacÃ­o: patrones ampliados
  (subtÃ­tulo "obtÃ©n funciones adicionales / get extra features", botÃ³n exacto
  "Descargar/Download" solo si es hoja con texto â‰¤12 chars), zonas protegidas
  (#pane-side, [role=listitem], [contenteditable], #main â€” NUNCA se tocan),
  colapso de ancestros solo si TODOS los hijos son promo/icono sin texto.
- El usuario probÃ³ EN VIVO durante el desarrollo: borrÃ³ todas las cuentas y la app
  auto-creÃ³ "Cuenta 1" con QR (el flujo nuevo funcionando en producciÃ³n real).

## Estado de verificaciÃ³n (con capturas reales de ventana)

- ARRANQUE VIRGEN: sin config â†’ auto-crea "Cuenta 1" (account-1) con QR
  (dot naranja) â€” nunca "default" (captura n1).
- TEMA: darkâ†’lightâ†’dark uniforme (capturas m2/m3, sin franjas mixtas);
  drawer reconstruido en cada apertura (siempre encima, estado fresco).
- SWITCH de cuentas: instantÃ¡neo por parking position-only (<350ms medidos).
- "+": en flujo bajo la Ãºltima cuenta (sube/baja); crea y abre su QR al
  instante; scroll del rail con rueda + indicador "hay mÃ¡s".
- DELETE por clic-derecho-armado con X roja visible + perfil borrado.
- BANDEJA: "Mostrar <cuenta>" restaura la ventana; menÃº dinÃ¡mico (set_menu);
  clic izquierdo en el icono trae la ventana; "Ocultar ventanas" oculta todo.
- AUTOSTART: POST prefs autostart escribe/borra HKCU Run "Zapdude" âœ“.
- LANDING del QR 100% limpio (banner de descarga eliminado por completo â€”
  fix triple: flags por script, cleanText de tokens ic-*, collapse del card
  por ancestro-promo con presupuesto de 400 chars).
- Guard de instancia Ãºnica: 2Âª instancia sale sola; la original sigue viva.
- QUIT desde API: proceso termina limpio.
- POP-OUT de llamadas: handler on_new_window instalado (comparte
  environment/sesiÃ³n) â€” PENDIENTE prueba manual del usuario.
- Tests 14/14. NSIS `Zapdude_1.0.0_x64-setup.exe` (2.81 MiB) instalado en
  silencio en la mÃ¡quina del usuario.

## PENDIENTE (bughunt activo del usuario â€” atender feedback con capturas)

- **Prueba EN VIVO del usuario** (prioridad): escanear QR, agregar varias
  cuentas, scroll del rail, tema, bandeja (mostrar/ocultar/clic izquierdo),
  borrado con X, pop-out de llamadas, autostart. Cualquier bug â†’ captura.
- **RediseÃ±ar los eventos push** (notificaciones â†’ API/webhook) de forma
  segura: hoy la API pasiva funciona (status/chats/send/open-chat) pero SIN
  eventos espontÃ¡neos; idealmente CDP/WebMessageReceived o polling del host.
- Iconos del rail estilo original (llamadas, favoritos, canales, etc.) y
  tabs adicionales (Canales, favoritos, mute, descargas, donar) â€” decidir
  alcance cuando el usuario lo pida.
- Workflow CI (GitHub Actions) para compilar Windows/macOS/Linux gratis en
  releases pÃºblicos (el usuario lo aprobÃ³ como plan para Mac â€” hacerlo tras
  el push a GitHub). Regenerar icon.icns para mac en el CI.
- Publicar en GitHub: repo sin crear aÃºn; pasos en el README/reportes
  (create repo â†’ git remote add â†’ push main --tags â†’ release con el NSIS).
- El tema NO toca la pÃ¡gina de WhatsApp (cada cuenta usa el tema de su
  WhatsApp); si el usuario pide forzar el tema de WhatsApp, se harÃ­a vÃ­a
  inyecciÃ³n en el localStorage de su UI.

## Comandos Ãºtiles

```powershell
# Build + tests + instalador
cd C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
cargo build; cargo test
.\tools\tauri.exe build --bundles nsis   # instalador en target\release\bundle\nsis

# Matar instancias previas antes de rebuild (error os error 5 si no)
Get-Process -Name zapdude | Stop-Process -Force

# Limpiar SIEMPRE la env var de bisecciÃ³n antes de probar producciÃ³n
Remove-Item Env:ZAPDUKE_WEBVIEW_OPTS -ErrorAction SilentlyContinue

# Smoke API
curl http://127.0.0.1:29977/api/healthz
curl http://127.0.0.1:29977/api/accounts
curl -X POST http://127.0.0.1:29977/api/accounts   # crear cuenta (como el "+")
```

## Notas de entorno

- PowerShell: cargo/tauri escriben a stderr â†’ ruido `NativeCommandError` cosmÃ©tico;
  capturar con `$o = cmd 2>&1; $LASTEXITCODE`.
- **El mismo shell de PowerShell PERSISTE env vars entre comandos** â€” la env var de
  debug `ZAPDUKE_WEBVIEW_OPTS` contaminÃ³ varias corridas; limpiarla SIEMPRE.
- `curl.exe` (no alias de PS). Para POST con body JSON usar archivos o Invoke-RestMethod.
- git: `C:\Program Files\Git\bin\git.exe` (PATH puede no estar refrescado).
- Usuario habla espaÃ±ol; responder en espaÃ±ol.

