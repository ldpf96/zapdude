# Arquitectura — ZapZap (Rust)

Documento vivo. Describe los componentes y decisiones de diseño del fork
Rust de ZapZap (créditos: Rafael Tosta, ver NOTICE.md).

## Vista general

```
┌────────────────────────────────────────────────────────────────────┐
│  zapdude.exe (Tauri 2 + WebView2)                                  │
│                                                                    │
│  ┌──────────────┐   ┌───────────────┐   ┌──────────────────────┐   │
│  │ ventana      │   │ ventana       │   │ ventana              │   │
│  │ account-     │   │ account-work  │   │ account-…            │   │
│  │ default      │   │ (oculta)      │   │ (oculta)             │   │
│  │              │   │               │   │                      │   │
│  │ perfil web-  │   │ perfil web-   │   │ perfil webview/work  │   │
│  │ view/default │   │ view/work     │   │                      │   │
│  └──────┬───────┘   └───────┬───────┘   └──────────┬───────────┘   │
│         │ bridge JS         │                      │               │
│         ▼                   ▼                      ▼               │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ page_ops (1 hilo poller/beacons por cuenta, 30 ms)           │   │
│  │ transporte: eval_with_callback (primario) + beacon hash URL  │   │
│  └───────────────────────────────┬──────────────────────────────┘   │
│                                  │                                  │
│  ┌───────────────────────┐   ┌───▼──────────────────────────────┐   │
│  │ tray (bandeja)        │   │ API axum  (loopback 127.0.0.1)   │   │
│  │ Show/Hide/Exit        │   │ REST + SSE + webhook (ureq v3)   │   │
│  └───────────────────────┘   └──────────────────────────────────┘   │
│                                                                    │
│  config.json (settings) ── personas: ventanas ── agentes: API      │
└────────────────────────────────────────────────────────────────────┘
```

## Módulos

| Módulo | Responsabilidad |
|---|---|
| `src/main.rs` | arranque, carga de settings, `on_window_event` (CloseRequested→hide en cuentas), `SHARED` global |
| `src/config.rs` | `config.json`: `ApiConfig` + `AccountConfig[]`, sanitización (ids `alnum-_`, ≤48 chars, dedupe, fallback "default"), persistencia atómica, BOM-tolerante |
| `src/window.rs` | crea ventanas por cuenta con perfil aislado, UA + args Chromium (`--disable-background-timer-throttling…`), `initialization_script`, permisos, bandeja y router de menús |
| `src/api/server.rs` | servidor axum en hilo "zapdude-api" (tokio dedicado, apagado select/2s), rutas planas `/api/*`, autorización, rate limit, estados de sesión |
| `src/api/page_ops.rs` | registry de bridges por cuenta (`page_for`), evaluaciones JS con timeout 15 s, flows open-chat/send/get-status/list-chats, beacons entrantes→eventos etiquetados |
| `src/api/protocol.rs` | API key, errores HTTP tipados, rate limiter anillo de 60/min, HMAC firma webhook |
| `src/api/events.rs` | ring buffer 200 + broadcast para SSE |
| `scripts/api_page_bridge.js` | residente por documento top-level: `window.__zapZapApiV1` (pageInfo/composerInfo/sendCurrent/chats), hook de Notification→eventos, beacon `#__zapdude=` |
| `scripts/client_hints_fix.js` | alinea `navigator.userAgentData`/`userAgent` con Chrome estable (hacia llamadas de WhatsApp Web) |

## Decisiones clave

1. **Multi-cuenta real por aislamiento de perfil:** cada cuenta tiene su
   `data_directory` WebView2 (sesiones, localStorage, Service Worker
   independientes). Identidad estable = `id` alfanumérico persistido;
   `label` es editable para humanos. La primera cuenta habilitada arranca
   visible; el resto oculto recibiendo notificaciones.
2. **Nunca se rompe:** la página externa (WhatsApp) podría cargar mal —
   el puente es defensivo: cada consulta puede degradar a error de API
   (`unavailable`, `degraded`) pero la ventana, la bandeja y la API
   siguen vivas. Los errores de poller se ignoran y se reintentan.
3. **Transporte página↝host:** `eval_with_callback` (WebView2 moderno;
   ojo: en Windows el resultado llega **JSON doble-codificado si es
   String** → se desenrolla en `evaluate`). Reserva/eventos: beacon en el
   hash de la URL actual `#__zapdude=<json>` polled cada 30 ms; el host
   limpia la parte del hash tras consumir.
4. **API:** loopback por defecto (`127.0.0.1`), clave obligatoria
   (excepto `healthz`), 60 req/min, cuerpo ≤ 1 MB, eventos con ring 200 +
   broadcast SSE (keep-alive 15 s), webhook opcional firmado HMAC-SHA256
   (`X-ZapZap-Signature: sha256=…`) enviado con `ureq` v3.
5. **Apagado limpio:** `ServerHolder` con `AtomicBool` vigilado en
   `tokio::select!` y `shutdown_timeout(2s)`; la tray "Exit ZapZap"
   apaga servidor y luego `app.exit(0)`. Cerrar ventanas de cuenta solo
   las oculta (notificaciones siguen).
6. **Identidad de llamadas:** UA Chrome estable + `client_hints_fix.js`
   en el frame superior. Limitación honesta: no se pueden reescribir las
   cabeceras HTTP `Sec-CH-UA` reales (Tauri solo toca `tauri://`);
   el *feature gate* de WhatsApp Web lee la identidad JS, pero WhatsApp
   puede pedir "actualizar" en casos raros.
7. **Args Chromium:** además de los disable default de WebView2
   (msWebOOUI/msPdfOOUI/msSmartScreenProtection), se congelan throttling
   de timers y render en segundo plano para que las cuentas ocultas
   sigan recibiendo mensajes y las llamadas suenen.

## Flujo de un mensaje saliente (`/api/message`)

1. Auth (X-API-Key/Bearer) + rate limit + body limit.
2. `resolve_account_id` (parámetro `account` > primera habilitada).
3. `PageBridge::send_message(phone, text)`:
   - normaliza E.164, navega `https://web.whatsapp.com/send?phone=<digits>
     [&text=<enc>]` (prefill solo ≤500 chars; sin prefill → inyección in-page),
   - espera composer visible (≤15 s),
   - `sendCurrent`: pegado resilient (Lexical-aware) + Enter + fallback botón,
4. Resultado JSON (o `ApiError` HTTP tipado).

## Seguridad

- Clave generada aleatoria al primer arranque; toda escritura de config
  queda en memoria limpiada de valores inválidos.
- API **off por defecto**. `exposeNetwork` false → bind loopback.
- Webhook solo URLs http(s); firma HMAC si hay secret; nunca se envía la
  clave de API.
- Sin nsis `perMachine`: instalador `currentUser` (sin admin).
