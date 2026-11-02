# SPDX-License-Identifier: GPL-3.0-only
# Enables or disables automatic firmware loading for the current user only:
# a Startup-folder shortcut that runs autoload.ps1 hidden. No admin, service,
# scheduled task, registry or driver change.
[CmdletBinding()]
param(
    [switch]$Install,
    [switch]$Remove,
    [string]$Firmware
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$base = Join-Path $env:LOCALAPPDATA 'SamsungCamera'
$stored = Join-Path $base 'firmware\FalconFW.bin'
$shortcut = Join-Path ([Environment]::GetFolderPath('Startup')) 'Samsung Camera Autoload.lnk'
$script = Join-Path $PSScriptRoot 'autoload.ps1'
$exe = Join-Path $PSScriptRoot 'samsung-camera-windows.exe'
$usbSuspend = Join-Path $PSScriptRoot 'usb-suspend.ps1'

function Stop-Watcher {
    Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe'" | Where-Object {
        $_.CommandLine -and $_.CommandLine -like '*autoload.ps1*' -and $_.ProcessId -ne $PID
    } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
}

try {
    if ($Install -eq $Remove) { throw 'Choose either -Install or -Remove.' }
    if ($Remove) {
        Stop-Watcher
        & powershell.exe -NoProfile -File $usbSuspend -Restore 2>$null | Out-Null
        if (Test-Path -LiteralPath $shortcut) { Remove-Item -LiteralPath $shortcut -Force }
        Write-Output 'Automatic firmware loading disabled. The stored firmware file was kept.'
        exit 0
    }
    if ($Firmware) {
        & $exe inspect $Firmware | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'The given firmware is not the pinned VG-STC3000 image; nothing changed.' }
        [void][System.IO.Directory]::CreateDirectory((Split-Path -Parent $stored))
        Copy-Item -LiteralPath $Firmware -Destination $stored -Force
    }
    & $exe inspect $stored | Out-Null
    if ($LASTEXITCODE -ne 0) { throw ('Firmware not found at ' + $stored + '. Pass -Firmware C:\path\to\FalconFW.bin.') }
    # The camera resets itself minutes after Windows suspends its USB port.
    & powershell.exe -NoProfile -File $usbSuspend -Disable
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut($shortcut)
    $link.TargetPath = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $link.Arguments = '-NoProfile -WindowStyle Hidden -File "' + $script + '"'
    $link.WorkingDirectory = $PSScriptRoot
    $link.Save()
    Stop-Watcher
    Start-Process -WindowStyle Hidden -FilePath $link.TargetPath -ArgumentList $link.Arguments
    Write-Output 'Automatic firmware loading enabled: it starts at sign-in and loads the firmware whenever the cold camera appears.'
    Write-Output ('Log: ' + (Join-Path $base 'autoload.log') + '. Disable with AUTOLOAD-REMOVE.cmd.')
    exit 0
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
