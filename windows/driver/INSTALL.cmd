@echo off
setlocal
powershell.exe -NoProfile -File "%~dp0install.ps1" -Install -ConfirmInstall
set "RESULT=%ERRORLEVEL%"
if not "%RESULT%"=="0" echo Driver installation stopped. Do not disable signature enforcement or Windows security.
pause
exit /b %RESULT%
