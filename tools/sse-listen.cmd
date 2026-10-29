@echo off
rem Zapdude SSE stream test listener. The API key is read from the app's
rem config file at runtime - nothing secret lives in this repository file.
rem Usage: run-me.cmd while Zapdude is running; it captures ~16 s of the
rem event stream into sse-out.tmp so the host can verify live delivery.
for /f "usebackq delims=" %%k in (`powershell -NoProfile -Command "(Get-Content -Raw -LiteralPath (Join-Path $env:APPDATA 'app.zapzap.desktop\config.json') | ConvertFrom-Json).api.apiKey"`) do set "ZKEY=%%k"
cd /d C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
curl -s -N -m 16 http://127.0.0.1:29977/api/events/stream --header "X-API-Key: %ZKEY%" -o "C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\sse-out.tmp"
