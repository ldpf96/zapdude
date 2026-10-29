@echo off
cd /d C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
set ZAPDUDE_DEBUG_EVAL=1
target\debug\zapdude.exe > C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\zapdude-console.log 2>&1
