# SPDX-License-Identifier: GPL-3.0-only
# Per-user watcher: loads the pinned firmware into the camera RAM whenever the cold
# camera (04e8:205c, bound to WinUSB) appears. It never touches an initialized
# camera (04e8:205d) and never installs anything. Stop it with AUTOLOAD-REMOVE.cmd.
[CmdletBinding()]
param(
    [string]$Firmware = (Join-Path $env:LOCALAPPDATA 'SamsungCamera\firmware\FalconFW.bin'),
    [int]$PollSeconds = 2,
    [switch]$Once
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$exe = Join-Path $PSScriptRoot 'samsung-camera-windows.exe'
$logPath = Join-Path $env:LOCALAPPDATA 'SamsungCamera\autoload.log'

function Write-Log([string]$Text) {
    try {
        if ((Test-Path -LiteralPath $logPath) -and (Get-Item -LiteralPath $logPath).Length -gt 200KB) {
            Remove-Item -LiteralPath $logPath -Force
        }
        Add-Content -LiteralPath $logPath -Value ((Get-Date -Format 's') + ' ' + $Text)
    } catch { }
}

function Get-State {
    # Native stderr (e.g. no camera present) must not become a terminating error.
    $ErrorActionPreference = 'Continue'
    $lines = @(& $exe inventory 2>&1 | ForEach-Object { "$_" })
    if ($LASTEXITCODE -ne 0) { return 'none' }
    $devices = @($lines | Where-Object { $_ -like '*"kind":"device"*' } | ForEach-Object { $_ | ConvertFrom-Json })
    if (@($devices | Where-Object { $_.phase -eq 'runtime' }).Count -gt 0) { return 'runtime' }
    $boot = @($devices | Where-Object { $_.phase -eq 'boot' })
    if ($boot.Count -eq 0) { return 'none' }
    if (@($boot | Where-Object { $_.service -ieq 'WINUSB' }).Count -gt 0) { return 'boot-ready' }
    return 'boot-unbound'
}

$mutex = New-Object System.Threading.Mutex($false, 'Local\SamsungCameraAutoload')
if (-not $mutex.WaitOne(0)) { exit 0 }
try {
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'samsung-camera-windows.exe is missing.' }
    & $exe inspect $Firmware | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Firmware file is missing or is not the pinned VG-STC3000 image.' }
    Write-Log 'watcher started'
    $failures = 0
    $last = ''
    while ($true) {
        $state = Get-State
        if ($state -ne $last) { Write-Log ('state: ' + $state); $last = $state }
        $wait = $PollSeconds
        if ($state -eq 'boot-ready') {
            $ErrorActionPreference = 'Continue'
            $output = @(& $exe load $Firmware 2>&1 | ForEach-Object { "$_" })
            $loadExit = $LASTEXITCODE
            $ErrorActionPreference = 'Stop'
            if ($loadExit -eq 0) {
                Write-Log ('firmware loaded: ' + ($output -join ' '))
                $failures = 0
                $wait = 5
            } else {
                $failures++
                Write-Log ('load failed (' + $failures + '): ' + ($output -join ' '))
                $wait = if ($failures -ge 3) { 120 } else { 15 }
                if ($failures -ge 3) { $failures = 0 }
            }
            $last = ''
        }
        if ($Once) { break }
        Start-Sleep -Seconds $wait
    }
} catch {
    Write-Log ('fatal: ' + $_.Exception.Message)
    Write-Error $_.Exception.Message
    exit 1
} finally {
    $mutex.ReleaseMutex()
    $mutex.Dispose()
}
