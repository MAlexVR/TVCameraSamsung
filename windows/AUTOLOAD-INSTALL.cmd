@echo off
setlocal
rem Enables per-user automatic firmware loading. Pass -Firmware C:\path\FalconFW.bin the first time if needed.
powershell.exe -NoProfile -File "%~dp0autoload-setup.ps1" -Install %*
set "RESULT=%ERRORLEVEL%"
pause
exit /b %RESULT%
