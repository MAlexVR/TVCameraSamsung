@echo off
setlocal
cd /d "%~dp0"
if errorlevel 1 exit /b 1
where cargo >nul 2>&1
if errorlevel 1 (
  echo Rust and the Windows linker are required. See README.md. Nothing was installed.
  exit /b 1
)
cargo test --offline --locked --target-dir "%~dp0build"
if errorlevel 1 exit /b 1
cargo build --offline --locked --release --target-dir "%~dp0build"
if errorlevel 1 exit /b 1
copy /y "%~dp0build\release\samsung-camera-windows.exe" "%~dp0samsung-camera-windows.exe" >nul
if errorlevel 1 exit /b 1
echo Built the Windows camera utilities. A signed WinUSB catalog is still required for driver installation.
