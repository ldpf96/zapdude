# NOTICE

Zapdude — cliente de escritorio de WhatsApp Web multi-cuenta con API local
para automatización, escrito en Rust con Tauri 2 y WebView2.

## Créditos

Este proyecto es una **reescritura en Rust** de
[ZapZap](https://github.com/rafatosta/zapzap), creado originalmente por
**Rafael Tosta** (@rafatosta) en Python (GTK + WebKitGTK).

- Proyecto original: https://github.com/rafatosta/zapzap
- Licencia original: GPL-3.0 (Free Software Foundation, versión 3)
- Autor original: Rafael Tosta

Toda la arquitectura aquí presente (multi-cuenta, bandeja del sistema con
lista de cuentas, identidad del navegador alineada a Chrome para habilitar
llamadas, y API local con autenticación por clave) está inspirada en o
derivada del trabajo original. El código es nuevo, pero se distribuye como
**obra derivada** de ZapZap conforme a la **GNU Affero General Public
License v3.0** (ver `LICENSE`).

## Aviso legal adicional

ZapZap (y este fork) **no están afiliados, patrocinados ni respaldados** por
WhatsApp LLC ni Meta Platforms, Inc. "WhatsApp" es una marca de WhatsApp LLC.
Este software automatiza la interfaz web pública que el propio usuario inicia
sesión; el uso que le des (incluido con agentes de IA) es tu responsabilidad
y puede estar sujeto a los términos de servicio de WhatsApp.

## Resumen de condiciones AGPL-3.0 (no sustituye al texto legal)

- Copiar, modificar y redistribuir es libre **siempre que** la obra derivada
  conserve la misma licencia y la atribución a Rafael Tosta.
- Si ejecutas el servicio modificado en un servidor y otras personas interactúan
  con él de forma remota (por ejemplo, mediante la API local/red), debes
  ofrecerles el código fuente de tu versión (sección 13 de AGPL-3.0).
