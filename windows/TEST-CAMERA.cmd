@echo off
setlocal
cd /d "%~dp0"
if errorlevel 1 exit /b 1
powershell.exe -NoProfile -File "%~dp0test-camera.ps1" %*
set "RESULT=%ERRORLEVEL%"
pause
exit /b %RESULT%
