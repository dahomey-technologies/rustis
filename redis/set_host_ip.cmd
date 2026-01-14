@echo off
for /f "tokens=2 delims=:" %%a in ('ipconfig ^| findstr /C:"IPv4" ^| findstr /V "172."') do (SET HOST_IP=%%a)
set HOST_IP=%HOST_IP: =%
