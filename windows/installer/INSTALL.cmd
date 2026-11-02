@echo off
setlocal
powershell.exe -NoProfile -File "%~dp0install.ps1" -Install -ConfirmInstall
set "RESULT=%ERRORLEVEL%"
if not "%RESULT%"=="0" echo Installation stopped. Do not disable Windows security to run this package.
pause
exit /b %RESULT%
