; Custom NSIS hooks for the Zapdude installer (referenced from tauri.conf.json).

!macro NSIS_HOOK_POSTUNINSTALL
  ; Full cleanup: remove config and every WebView2 profile on uninstall.
  RMDir /r "$APPDATA\app.zapzap.desktop"
!macroend
