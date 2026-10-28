# Zapdude

**Zapdude** es un cliente de escritorio de WhatsApp Web multi-cuenta para
Windows, escrito en **Rust** (Tauri 2 + WebView2). Todas tus cuentas de
WhatsApp en una sola ventana, con una API local para que personas y agentes
de IA puedan operarlas.

**Proyecto original de [ldpf96](https://github.com/ldpf96)** — todo el código
fue escrito desde cero para este proyecto (Rust/Tauri/WebView2). La *idea*
está inspirada en el concepto del cliente
[ZapZap](https://github.com/rafatosta/zapzap) de Rafael Tosta; no se copia,
porteó ni reutiliza nada de su código. Detalles en [`NOTICE.md`](NOTICE.md).
Licencia: **AGPL-3.0**.

![Windows 10/11](https://img.shields.io/badge/Windows-10%2F11-0078d7) ![Rust](https://img.shields.io/badge/Rust-stable-dea584) ![Tauri 2](https://img.shields.io/badge/Tauri-2-FFC131) ![Licencia](https://img.shields.io/badge/Licencia-AGPL--3.0-blue)

## Características

- **Multi-cuenta vital** — rail lateral con logo, un avatar por
  cuenta y "+". Agregar una cuenta no pide formularios: abre su QR al
  instante. Cada cuenta tiene un **perfil WebView2 aislado**, así que las
  sesiones no se mezclan.
- **Cambio de cuenta instantáneo** — las cuentas inactivas se "aparcan"
  fuera de la ventana a tamaño completo y el cambio es solo un movimiento de
  posición: WhatsApp no se re-renderiza (medido: pintado completo en <350 ms).
- **Arranque rápido con N cuentas** — solo se carga WhatsApp Web de la
  cuenta que abres; las demás nacen cuando las usas por primera vez.
- **Opciones a pantalla completa** (botón del logo): configuración de la API
  con estado en vivo, documentación del agente de IA y "Acerca de" — todo en
  la misma ventana, sin ventanas auxiliares.
- **Bandeja del sistema** con "Mostrar <cuenta>" por cuenta, ocultar todo y
  salir.
- **Llamadas de WhatsApp Web** — identidad Chrome estable (UA + parche de
  *client hints* en JS) y permisos aprobados para Micrófono, Cámara,
  Compartir pantalla, Notificaciones y Portapapeles.
- **Limpieza de promos** — el banner "Download WhatsApp for Windows" del
  landing y la tarjeta de descarga del panel vacío se eliminan solos.
- **Sin terminal, sin fricción** — `windows_subsystem`, ventana única que se
  oculta a la bandeja, instancia única (mutex), instalador NSIS.

## Instalación (Windows 10/11)

Descarga `Zapdude_1.0.0_x64-setup.exe` (releases) y ejecútalo. Modo
`currentUser` (no requiere admin), selector de idioma español/inglés. Al
desinstalar se borran también los datos de la app.

O compílalo tú mismo:

```powershell
cd zapdude-rs
.\tools\tauri.exe build --bundles nsis   # instalador en target\release\bundle\nsis
```

## Configuración

`%APPDATA%\app.zapzap.desktop\config.json` (se crea solo en el primer
arranque):

```json
{
  "api": {
    "enabled": false,
    "port": 29977,
    "apiKey": "zpk_...generada-automaticamente...",
    "exposeNetwork": false,
    "webhookUrl": "",
    "webhookSecret": ""
  },
  "accounts": [
    { "id": "trabajo", "label": "Trabajo", "enabled": true }
  ]
}
```

- **API desactivada por defecto** (privacidad): actívala desde las opciones
  (logo → Configuración API) o con `"enabled": true`.
- La `apiKey` se genera aleatoria al primer arranque.
- `exposeNetwork: true` escucha en `0.0.0.0` (¡solo si sabes lo que haces!).

## API local (v1)

Base `http://127.0.0.1:<puerto>/api` · autenticación `X-API-Key` (o
`Authorization: Bearer`) · 60 peticiones/minuto · cuerpo ≤ 1 MB.

| Método | Ruta | Descripción |
|---|---|---|
| GET  | `/api/healthz` | Sondeo de disponibilidad (sin clave) |
| GET  | `/api/accounts` | Cuentas configuradas y su estado |
| POST | `/api/accounts` | Agrega cuenta y abre su QR |
| DELETE | `/api/accounts/{id}` | Elimina una cuenta (y su perfil) |
| POST | `/api/accounts/{id}/open` | Muestra esa cuenta en la ventana |
| GET  | `/api/status?account=<id>` | App + página + `accounts[]` |
| POST | `/api/open-chat` | `{"account","phone","text?"}` |
| POST | `/api/message` | `{"account","phone","message"}` |
| GET  | `/api/chats?limit=20&account=<id>` | Lista visible de chats (best-effort) |
| GET  | `/api/events?since=<id>` | Historial reciente de eventos |
| SSE  | `/api/events/stream` | Eventos en vivo |
| GET/POST | `/api/webhook` | Receptor HTTPS de eventos (firma HMAC) |

Ejemplos:

```bash
# ¿Está viva la API?
curl http://127.0.0.1:29977/api/healthz

# Estado (session_state: detecting | scan_qr | logged_in)
curl -H "X-API-Key: $KEY" "http://127.0.0.1:29977/api/status"

# Enviar mensaje
curl -X POST -H "X-API-Key: $KEY" -H "Content-Type: application/json" \
  -d '{"account":"trabajo","phone":"34612345678","message":"Hola desde Rust"}' \
  http://127.0.0.1:29977/api/message

# Eventos en vivo (SSE)
curl -N -H "X-API-Key: $KEY" http://127.0.0.1:29977/api/events/stream
```

## Cómo funciona

Una sola ventana Tauri aloja varios webviews: el **rail** (76 px), el panel
de **opciones** (pantalla completa, creado bajo demanda), un *backdrop* de
bienvenida y un webview de WhatsApp Web por cuenta (perfiles WebView2
aislados en `%APPDATA%\app.zapzap.desktop\webview\<id>`).

Toda la interfaz (rail/opciones/bienvenida) la sirve el propio **servidor
API de loopback** (mismo origen): los `fetch` de la UI nunca sufren CORS ni
problemas de `file://`, y no se usa IPC de Tauri para la UI.

La automatización (leer chats, abrir conversaciones, enviar mensajes) usa un
bridge residente inyectado en la página con transporte por *beacon* en el
hash del URL; cada fallo degrada a un error de API sin romper la app.

```
zapdude-rs/
├── src/window.rs      # shell multi-webview + bandeja + ciclo de vida
├── src/api/           # servidor axum, protocolo, bridge de página, eventos
├── scripts/           # client-hints, bridge residente, limpieza de promos
├── ui/                # rail, opciones (drawer) y bienvenida
├── windows/           # hooks NSIS (limpieza al desinstalar)
└── docs/              # arquitectura y notas de sesión
```

## Desarrollo

```powershell
cargo run                               # debug
cargo test                              # pruebas unitarias
.\tools\tauri.exe build --bundles nsis  # release + instalador
```

Requiere el Rust toolchain estable y el WebView2 Runtime (incluido de
fábrica en Windows 11).

## Limitaciones conocidas

- WhatsApp no publica una API oficial: la automatización usa la página real
  y es **best-effort** ante cambios de su interfaz.
- Las cabeceras HTTP `Sec-CH-UA` no se pueden reescribir desde Tauri; la
  identidad Chrome se aplica en JS, que es lo que los *feature gates* de
  llamadas de WhatsApp Web leen.
- Las cuentas de fondo se cargan al primer uso: sus notificaciones en vivo
  empiezan cuando se abren por primera vez en la sesión.

## Licencia

**AGPL-3.0** — ver [`LICENSE`](LICENSE) y [`NOTICE.md`](NOTICE.md).
Código original de [ldpf96](https://github.com/ldpf96); la *idea* está
inspirada en ZapZap (Rafael Tosta), sin reutilizar su código. Zapdude no es
un producto oficial de WhatsApp ni de Meta, y no está afiliado a ellos.
