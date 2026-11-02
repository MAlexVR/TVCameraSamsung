@echo off
setlocal
cd /d "%~dp0"
if errorlevel 1 exit /b 1
if not exist "%~dp0samsung-camera-windows.exe" (
  echo No executable is present. This may be a source-only package. See README.md.
  exit /b 1
)
set "REPORT=%~dp0samsung-camera-windows-report-%RANDOM%-%RANDOM%.txt"
if exist "%REPORT%" (
  echo Report already exists; nothing was overwritten. Run again for a new name.
  exit /b 1
)
"%~dp0samsung-camera-windows.exe" inventory > "%REPORT%" 2>&1
set "INVENTORY_EXIT=%ERRORLEVEL%"
"%~dp0samsung-camera-windows.exe" probe >> "%REPORT%" 2>&1
set "PROBE_EXIT=%ERRORLEVEL%"
echo inventory_exit=%INVENTORY_EXIT% probe_exit=%PROBE_EXIT% >> "%REPORT%"
echo Report: "%REPORT%"
echo No firmware, capture, driver installation or LED writes were requested.
if not "%INVENTORY_EXIT%"=="0" exit /b 1
if not "%PROBE_EXIT%"=="0" exit /b 1
