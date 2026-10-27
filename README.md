# ZapZap (Rust)

Cliente de escritorio de WhatsApp Web escrito en **Rust** (Tauri 2 + WebView2),
fork/de reescritura de [ZapZap](https://github.com/rafatosta/zapzap) de
**Rafael Tosta** (ver `NOTICE.md`, licencia **AGPL-3.0**).

Requiere Windows 10/11 (WebView2, incluido de fábrica en Windows 11).

## Qué hace

- **Multi-cuenta (vital):** una ventana por cuenta de WhatsApp con
  **perfil WebView2 aislado** (datos en `%APPDATA%\app.zapzap.desktop\webview\<id>`).
  Todas las cuentas viven en la misma aplicación y reciben notificaciones
  aunque sus ventanas estén ocultas (cerrar la ventana solo la oculta).
- **Bandeja del sistema:** menú con "Mostrar <cuenta>" por cuenta,
  "Hide all windows" y "Exit ZapZap".
- **Llamadas de WhatsApp Web:** identidad alineada a Chrome estable
  (UA + parche de *client hints* en JS) y permisos ya aprobados para
  Micrófono, Cámara, Compartir pantalla, Notificaciones y Portapapeles,
  para que los puntos de entrada de llamada de voz/vídeo/pantalla
  funcionen como en el navegador.
- **API local para humanos y agentes de IA** (HTTP en loopback,
  `X-API-Key`), con eventos en vivo (SSE o *polling*) y webhook opcional
  firmado con HMAC.

## Instalador Windows (NSIS)

1. Genera el instalador (o descarga `target/release/bundle/nsis/…-setup.exe`):
   ```powershell
   cd zapdude-rs
   .\tools\tauri.exe build --bundles nsis
   ```
2. Ejecuta el `.exe` del instalador. Modo `currentUser` (no necesita admin),
   con selector de idioma (inglés/español).

## Configuración

`%APPDATA%\app.zapzap.desktop\config.json` (se crea solo en el primer arranque):

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
    { "id": "default", "label": "default", "enabled": true },
    { "id": "work",    "label": "Trabajo", "enabled": true }
  ]
}
```

- **API desactivada por defecto** (privacidad): pon `"enabled": true` y reinicia.
- La `apiKey` se genera aleatoria al primer arranque; câmbiala si quieres.
- `exposeNetwork: true` escucha en `0.0.0.0` (¡solo si sabes lo que haces!).
- Cada cuenta de `accounts` abre su propia ventana/perfil al iniciar.
  Los `id` admiten letras, números, `-` y `_`.

## API local (v1)

Base: `http://127.0.0.1:<puerto>/api` · Autenticación: cabecera
`X-API-Key: <clave>` o `Authorization: Bearer <clave>`.
Límites: 60 peticiones/minuto, cuerpo ≤ 1 MB.

| Método | Ruta | Descripción |
|---|---|---|
| GET  | `/api/healthz` | Sondeo de disponibilidad (sin clave) |
| GET  | `/api/status?account=<id>` | App + página + `accounts[]` de la cuenta |
| GET  | `/api/accounts` | Cuentas configuradas |
| POST | `/api/open-chat` | `{"account":"work","phone":"34612345678","text":"Hola"}` |
| POST | `/api/message` | `{"account":"work","phone":"34612…","message":"Texto"}` |
| GET  | `/api/chats?limit=20&account=<id>` | Lista visible de chats (best-effort) |
| GET  | `/api/events?since=<id>` | Historial reciente de eventos (todas las cuentas) |
| SSE  | `/api/events/stream` | Eventos en vivo para agentes/consumidores |
| GET/POST | `/api/webhook` | Receptor HTTPS de eventos (firmado `X-ZapZap-Signature: sha256=…`) |

Ejemplos:

```bash
# ¿Está viva la API?
curl http://127.0.0.1:29977/api/healthz

# Estado completo (enums de sesión: detecting | scan_qr | logged_in)
curl -H "X-API-Key: $KEY" "http://127.0.0.1:29977/api/status?account=default"

# Enviar mensaje desde la cuenta "work"
curl -X POST -H "X-API-Key: $KEY" -H "Content-Type: application/json" \
  -d '{"account":"work","phone":"34612345678","message":"Hola desde Rust"}' \
  http://127.0.0.1:29977/api/message

# Eventos en vivo (SSE)
curl -N -H "X-API-Key: $KEY" http://127.0.0.1:29977/api/events/stream
```

Los eventos entrantes (mensajes/notificaciones del bridge de página) llevan
siempre el campo `"account"` indicando la cuenta de origen.

## Limitaciones conocidas

- WhatsApp no publica API oficial: la automatización usa la página real y es
  **best-effort** ante cambios de su interfaz (los errores degradan a respuestas
  de API, nunca rompen la app).
- Las cabeceras HTTP `Sec-CH-UA` no se pueden reescribir desde Tauri (solo el
  contenido de `tauri://`); la identidad Chrome alineada se aplica en JS, que es
  lo que los *feature gates* de llamadas de WhatsApp Web leen. En casos raros
  WhatsApp puede pedir "actualizar".
- Compartir pantalla muestra el selector nativo de Windows (comportamiento
  esperado).
- Sin sesión iniciada el estado es `scan_qr`: escanea el QR en la ventana de la
  cuenta correspondiente.

## Desarrollo

```powershell
cargo run          # ejecutar en debug
cargo test         # 14 pruebas unitarias
.\tools\tauri.exe build --bundles nsis   # release + instalador
```

Estructura: `src/window.rs` (multi-ventana+bandeja), `src/api/` (servidor axum,
bridge de página JS, protocolo), `scripts/` (bridge residente + client-hints),
`docs/architecture.md` (arquitectura detallada).

## Licencia

AGPL-3.0 — ver `LICENSE` y `NOTICE.md` (crédito obligatorio a Rafael Tosta).
