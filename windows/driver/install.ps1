# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param(
    [switch]$Install,
    [switch]$Check,
    [switch]$ConfirmInstall
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-PlainPath([string]$Path) {
    $current = [System.IO.Path]::GetFullPath($Path)
    while ($current) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw 'Reparse points are not accepted in the driver package path.'
            }
        }
        $parent = [System.IO.Path]::GetDirectoryName($current)
        if ($parent -eq $current) { break }
        $current = $parent
    }
}

try {
    if ($Install -and $Check) { throw 'Choose either -Check or -Install.' }
    if ($ConfirmInstall -and -not $Install) { throw '-ConfirmInstall requires -Install.' }
    if (-not $Install -and -not $Check) {
        Write-Output 'Usage: powershell.exe -NoProfile -File driver\install.ps1 -Check'
        Write-Output 'Use -Install from an elevated terminal to install the signed WinUSB boot driver.'
        exit 0
    }
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or -not [Environment]::Is64BitOperatingSystem) {
        throw 'This driver package requires 64-bit Windows 10 or later.'
    }
    Assert-PlainPath $PSScriptRoot
    $inf = Join-Path $PSScriptRoot 'samsung-vg-stc3000.inf'
    $catalog = Join-Path $PSScriptRoot 'samsung-vg-stc3000.cat'
    foreach ($file in @($inf, $catalog)) {
        Assert-PlainPath $file
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            throw ('Missing driver package file: ' + [System.IO.Path]::GetFileName($file) + '. For the custom-INF route a signed catalog is required. Alternatively, use the inbox WinUsb Device selection and CONFIGURE-INBOX.cmd documented in README.md. No installation was attempted.')
        }
    }
    $definition = Get-Content -LiteralPath $inf -Raw
    foreach ($required in @('USB\VID_04E8&PID_205C', 'Include = winusb.inf', 'Needs   = WINUSB.NT', '{889992a2-9243-4ba5-beb7-7d0b029e11a0}')) {
        if (-not $definition.Contains($required)) { throw ('Driver INF is missing required entry: ' + $required) }
    }
    if ($definition -match '(?i)PID_205D') { throw 'The boot driver must never match initialized UVC device 04e8:205d.' }
    $signature = Get-AuthenticodeSignature -LiteralPath $catalog
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
        $null -eq $signature.SignerCertificate) {
        throw ('The driver catalog is not trusted and signed (status: ' + $signature.Status + '). No driver command was run. Sign it through the normal Windows driver-signing process; do not disable signature enforcement.')
    }
    if (-not $Install) {
        Write-Output ('Signed driver package checked. No system changes performed: ' + $inf)
        exit 0
    }
    $principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Administrator privileges are required. Right-click INSTALL.cmd and choose Run as administrator.'
    }
    if ($ConfirmInstall) {
        Write-Output 'This binds Microsoft WinUSB only to Samsung 04e8:205c (firmware boot).'
        Write-Output 'It does not replace the initialized 04e8:205d camera/audio drivers.'
        $answer = Read-Host 'Type INSTALL DRIVER to continue, or press Enter to cancel'
        if ($answer -cne 'INSTALL DRIVER') { Write-Output 'Cancelled; no system changes made.'; exit 0 }
    }
    $pnputil = Join-Path $env:SystemRoot 'System32\pnputil.exe'
    if (-not (Test-Path -LiteralPath $pnputil -PathType Leaf)) { throw 'Windows PnPUtil was not found.' }
    $output = & $pnputil /add-driver $inf /install 2>&1
    $result = $LASTEXITCODE
    $output | ForEach-Object { Write-Output $_ }
    if ($result -ne 0) { throw ('PnPUtil failed with exit code ' + $result + '. Preserve this output; do not disable Windows security.') }
    Write-Output 'Signed WinUSB boot driver installed. Reconnect the cold camera if PnPUtil did not bind it.'
    Write-Output 'Load firmware separately with samsung-camera-windows.exe load C:\path\to\FalconFW.bin'
    exit 0
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
