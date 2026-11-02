@echo off
setlocal
powershell.exe -NoProfile -File "%~dp0autoload-setup.ps1" -Remove
set "RESULT=%ERRORLEVEL%"
pause
exit /b %RESULT%
