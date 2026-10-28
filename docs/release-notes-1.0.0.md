# Zapdude 1.0.0 — primer release público

Cliente de escritorio de **WhatsApp Web multi-cuenta** para Windows, escrito
en **Rust** (Tauri 2 + WebView2) por **ldpf96**. Licencia **AGPL-3.0**.

## Instalación

1. Descarga `Zapdude_1.0.0_x64-setup.exe` (~2.81 MiB) de esta release.
2. Ejecuta el instalador (no requiere admin; instalador por usuario).
3. La app abre tu primera cuenta y su QR directamente. Escanea WhatsApp en
   tu teléfono → Dispositivos vinculados → Vincular un dispositivo.

## Lo que incluye

- Todas tus cuentas de WhatsApp en una sola ventana con rail lateral.
- Cambio de cuenta instantáneo (las cuentas inactivas quedan "aparcadas"
  a tamaño completo fuera de la ventana: WhatsApp no se re-renderiza).
- Perfiles WebView2 aislados por cuenta (las sesiones no se mezclan).
- Opciones a pantalla completa: tema claro/oscuro, iniciar con Windows,
  iniciar minimizado, renombrar cuentas, configuración de la API.
- Bandeja del sistema con "Mostrar <cuenta>", ocultar todo y salir.
- Llamadas de WhatsApp Web (identidad Chrome estable + permisos
  preaprobados de micrófono/cámara/pantalla).
- API local (loopback) para personas y agentes de IA: REST + SSE + webhook
  con firma HMAC (`X-Zapdude-Signature`). Desactivada por defecto.

## Notas

- Requiere Windows 10/11 con WebView2 Runtime (incluido de fábrica en
  Windows 11).
- Proyecto original de [ldpf96](https://github.com/ldpf96), inspirado en el
  concepto de [ZapZap](https://github.com/rafatosta/zapzap) (Rafael Tosta)
  con código 100% original. No está afiliado a WhatsApp ni a Meta.
- Código fuente: https://github.com/ldpf96/zapdude (AGPL-3.0).
