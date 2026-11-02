# SPDX-License-Identifier: GPL-3.0-only
# The initialized camera resets itself a few minutes after Windows suspends its USB
# port, losing the RAM firmware. This turns USB selective suspend off in the ACTIVE
# power plan only (no admin needed) and remembers the previous values for -Restore.
[CmdletBinding()]
param(
    [switch]$Disable,
    [switch]$Restore,
    [switch]$Check
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$subgroup = '2a737441-1930-4402-8d77-b2bebba308a3'
$setting = '48e6b7a6-50f5-4782-a5d4-53bb8f07e226'
$backup = Join-Path $env:LOCALAPPDATA 'SamsungCamera\usb-suspend-backup.json'

function Get-Values {
    $text = (& powercfg.exe /query SCHEME_CURRENT $subgroup $setting | Out-String)
    $hits = @([regex]::Matches($text, '0x[0-9a-fA-F]{8}') | ForEach-Object { [Convert]::ToInt32($_.Value.Substring(2), 16) })
    if ($hits.Count -lt 2) { throw 'Could not read the USB selective suspend setting.' }
    return @{ ac = $hits[$hits.Count - 2]; dc = $hits[$hits.Count - 1] }
}

function Set-Values([int]$Ac, [int]$Dc) {
    & powercfg.exe /setacvalueindex SCHEME_CURRENT $subgroup $setting $Ac
    if ($LASTEXITCODE -ne 0) { throw 'powercfg failed for the AC value.' }
    & powercfg.exe /setdcvalueindex SCHEME_CURRENT $subgroup $setting $Dc
    if ($LASTEXITCODE -ne 0) { throw 'powercfg failed for the battery value.' }
    & powercfg.exe /setactive SCHEME_CURRENT
}

try {
    if (@($Disable, $Restore, $Check | Where-Object { $_ }).Count -ne 1) {
        Write-Output 'Usage: usb-suspend.ps1 -Check | -Disable | -Restore'
        exit 0
    }
    $now = Get-Values
    if ($Check) {
        Write-Output ('USB selective suspend (1 = enabled): AC=' + $now.ac + ' battery=' + $now.dc)
        exit 0
    }
    if ($Disable) {
        if (-not (Test-Path -LiteralPath $backup)) {
            [void][System.IO.Directory]::CreateDirectory((Split-Path -Parent $backup))
            ($now | ConvertTo-Json) | Set-Content -LiteralPath $backup -Encoding ASCII
        }
        Set-Values 0 0
        Write-Output 'USB selective suspend disabled in the active power plan. Undo with -Restore.'
        exit 0
    }
    if (-not (Test-Path -LiteralPath $backup)) { throw 'No saved values; nothing to restore.' }
    $saved = Get-Content -LiteralPath $backup -Raw | ConvertFrom-Json
    Set-Values ([int]$saved.ac) ([int]$saved.dc)
    Remove-Item -LiteralPath $backup -Force
    Write-Output 'USB selective suspend restored to the previous values.'
    exit 0
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
