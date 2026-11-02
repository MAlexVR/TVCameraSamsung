@echo off
setlocal
rem Asks for administrator (UAC), then runs the guarded local-only driver install.
powershell.exe -NoProfile -Command "Start-Process -Verb RunAs -Wait -FilePath powershell.exe -ArgumentList '-NoProfile','-File','\"%~dp0local-driver.ps1\"','-Install'"
exit /b %ERRORLEVEL%
