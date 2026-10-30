@echo off
rem Zapdude DEV launcher: starts the debug binary with ZERO visible
rem windows - the old version blocked inside a cmd host, which showed the
rem user a black terminal every launch (they called it out: "vuelve turbio
rem que abra una CMD"). Now: powershell spawns the exe hidden, the cmd
rem dies instantly, logs still land in zapdude-console(.log / -err.log).
cd /d "%~dp0.."
powershell -NoProfile -Command "$env:ZAPDUDE_DEBUG_EVAL='1'; $env:RUST_BACKTRACE='1'; Start-Process -FilePath '.\target\debug\zapdude.exe' -WindowStyle Hidden -RedirectStandardOutput '.\zapdude-console.log' -RedirectStandardError '.\zapdude-console-err.log'"
