@echo off
setlocal
powershell.exe -NoProfile -File "%~dp0configure-inbox.ps1" -Configure -ConfirmConfigure
set "RESULT=%ERRORLEVEL%"
if not "%RESULT%"=="0" echo Configuration stopped. Verify the hardware ID and inbox WinUSB binding; do not disable Windows security.
pause
exit /b %RESULT%
