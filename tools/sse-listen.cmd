@echo off
cd /d C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs
curl -s -N -m 7 http://127.0.0.1:29977/api/events/stream --header "X-API-Key: zpk_846b54feb07ac04601129f85f3353cf73c6f9ac44c73c71e300740850251543d" -o "C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\sse-out.tmp"
