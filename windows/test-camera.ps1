# SPDX-License-Identifier: GPL-3.0-only
# Guided, read-only camera test. -Load is the only action that writes (device RAM).
[CmdletBinding()]
param(
    [string]$Firmware,
    [switch]$Load
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Step([string]$Text) { Write-Output ('== ' + $Text) }

try {
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or -not [Environment]::Is64BitOperatingSystem) {
        throw 'This test requires 64-bit Windows.'
    }
    if ($Load -and -not $Firmware) { throw '-Load requires -Firmware C:\path\to\FalconFW.bin.' }
    $exe = Join-Path $PSScriptRoot 'samsung-camera-windows.exe'
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'samsung-camera-windows.exe is missing from this folder.' }

    Step '1/4 Package integrity'
    $status = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'PACKAGE_STATUS.json') -Raw | ConvertFrom-Json
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { $actual = [System.BitConverter]::ToString($sha.ComputeHash([System.IO.File]::ReadAllBytes($exe))).Replace('-', '') } finally { $sha.Dispose() }
    if ($actual -ine $status.binary_sha256) { throw 'Executable SHA256 does not match PACKAGE_STATUS.json.' }
    Write-Output 'Executable matches the package manifest (detects corruption, not publisher authenticity).'

    Step '2/4 Camera inventory (read-only)'
    $lines = @(& $exe inventory 2>&1 | ForEach-Object { "$_" })
    if ($LASTEXITCODE -ne 0) { throw ('inventory failed: ' + ($lines -join ' ')) }
    $devices = @($lines | Where-Object { $_ -like '*"kind":"device"*' } | ForEach-Object { $_ | ConvertFrom-Json })
    $runtime = @($devices | Where-Object { $_.phase -eq 'runtime' })
    $boot = @($devices | Where-Object { $_.phase -eq 'boot' })
    if ($runtime.Count -gt 0) {
        Write-Output 'Camera is INITIALIZED (04e8:205d). No driver or firmware step is needed.'
        Write-Output 'Next: open the Windows "Camera" app and select the Samsung camera. Report what you see.'
        exit 0
    }
    if ($boot.Count -eq 0) {
        Write-Output 'No Samsung 04e8:205c/205d device found. Connect the camera, wait 10 seconds, run this again.'
        exit 1
    }
    Write-Output 'Camera is in COLD BOOT state (04e8:205c); firmware must be loaded into RAM.'

    Step '3/4 WinUSB binding of the boot device'
    if (@($boot | Where-Object { $_.service -ieq 'WINUSB' }).Count -eq 0) {
        Write-Output 'The cold camera is not bound to WinUSB.'
        Write-Output 'Fix: run driver\LOCAL-DRIVER.cmd (see driver\README.md, option C) or select the inbox driver (option A).'
        exit 1
    }
    Write-Output 'Samsung 04e8:205c is bound to WinUSB.'

    Step '4/4 Firmware'
    if (-not $Firmware) {
        Write-Output 'Ready. Provide your own FalconFW.bin and run:'
        Write-Output '  powershell.exe -NoProfile -File test-camera.ps1 -Firmware C:\path\to\FalconFW.bin -Load'
        exit 0
    }
    & $exe inspect $Firmware
    if ($LASTEXITCODE -ne 0) { throw 'Firmware validation failed; nothing was sent to the camera.' }
    if (-not $Load) {
        Write-Output 'Firmware is valid. Add -Load to upload it to the camera RAM.'
        exit 0
    }
    Write-Output 'This uploads firmware to the camera RAM (lost on power cycle). It installs no driver.'
    $answer = Read-Host 'Type LOAD to continue, or press Enter to cancel'
    if ($answer -cne 'LOAD') { Write-Output 'Cancelled; nothing sent.'; exit 0 }
    & $exe load $Firmware
    if ($LASTEXITCODE -ne 0) { throw 'Firmware load failed. Preserve the output above and report it.' }
    Write-Output 'Camera re-enumerated as 04e8:205d. Open the Windows "Camera" app to test video and audio.'
    exit 0
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
