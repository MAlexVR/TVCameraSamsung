@echo off
setlocal
rem Asks for administrator (UAC), then removes the local driver package and its certificate trust.
powershell.exe -NoProfile -Command "Start-Process -Verb RunAs -Wait -FilePath powershell.exe -ArgumentList '-NoProfile','-File','\"%~dp0local-driver.ps1\"','-Remove'"
exit /b %ERRORLEVEL%
