# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param(
    [switch]$Configure,
    [switch]$Check,
    [switch]$ConfirmConfigure
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$interfaceGuid = '{889992a2-9243-4ba5-beb7-7d0b029e11a0}'
$bootRoot = 'HKLM:\SYSTEM\CurrentControlSet\Enum\USB\VID_04E8&PID_205C'
$runtimeRoot = 'HKLM:\SYSTEM\CurrentControlSet\Enum\USB\VID_04E8&PID_205D'

function Get-WinUsbInstances([string]$Root) {
    if (-not (Test-Path -LiteralPath $Root)) { return @() }
    return @(Get-ChildItem -LiteralPath $Root | Where-Object {
        $service = (Get-ItemProperty -LiteralPath $_.PSPath -Name Service -ErrorAction SilentlyContinue).Service
        $null -ne $service -and $service -ieq 'WinUSB'
    })
}

try {
    if ($Configure -and $Check) { throw 'Choose either -Check or -Configure.' }
    if ($ConfirmConfigure -and -not $Configure) { throw '-ConfirmConfigure requires -Configure.' }
    if (-not $Configure -and -not $Check) {
        Write-Output 'Usage: powershell.exe -NoProfile -File driver\configure-inbox.ps1 -Check'
        Write-Output 'After manually selecting the inbox WinUsb Device driver, use -Configure as administrator.'
        exit 0
    }
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or -not [Environment]::Is64BitOperatingSystem) {
        throw 'This utility requires 64-bit Windows 10 or later.'
    }
    if (@(Get-WinUsbInstances $runtimeRoot).Count -ne 0) {
        throw 'Initialized camera 04e8:205d is bound to WinUSB. Restore the Microsoft UVC driver before continuing.'
    }
    $instances = @(Get-WinUsbInstances $bootRoot)
    if ($instances.Count -eq 0) {
        throw 'Boot device 04e8:205c is not bound to WinUSB. In Device Manager, select Update driver > Browse my computer > Let me pick > Universal Serial Bus devices > WinUsb Device. No registry changes made.'
    }
    if ($instances.Count -ne 1) {
        throw 'More than one 04e8:205c device is bound to WinUSB; refusing ambiguous configuration.'
    }
    $instancePath = $instances[0].PSPath
    $parameters = Join-Path $instancePath 'Device Parameters'
    $property = $null
    $existing = @()
    if (Test-Path -LiteralPath $parameters) {
        $property = Get-ItemProperty -LiteralPath $parameters -Name DeviceInterfaceGUIDs -ErrorAction SilentlyContinue
        if ($null -ne $property) { $existing = @($property.DeviceInterfaceGUIDs) }
    }
    foreach ($value in $existing) {
        $parsed = [Guid]::Empty
        if ($value -isnot [string] -or -not [Guid]::TryParse($value, [ref]$parsed)) {
            throw 'DeviceInterfaceGUIDs contains an invalid value; no registry changes made.'
        }
    }
    $configured = @($existing | Where-Object { $_ -ieq $interfaceGuid }).Count -ne 0
    if (-not $Configure) {
        Write-Output 'Samsung 04e8:205c is bound to the inbox WinUSB driver.'
        Write-Output ('Loader interface configured: ' + $configured)
        if (-not $configured) { Write-Output 'Run CONFIGURE-INBOX.cmd as administrator, then reconnect the cold camera.' }
        exit 0
    }
    $principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Administrator privileges are required. Right-click CONFIGURE-INBOX.cmd and choose Run as administrator.'
    }
    if ($ConfirmConfigure) {
        Write-Output 'This adds only the Samsung loader interface GUID to the single 04e8:205c device already bound to WinUSB.'
        Write-Output 'Existing interface GUIDs are preserved. Runtime 04e8:205d is not modified.'
        $answer = Read-Host 'Type CONFIGURE to continue, or press Enter to cancel'
        if ($answer -cne 'CONFIGURE') { Write-Output 'Cancelled; no registry changes made.'; exit 0 }
    }
    $currentService = (Get-ItemProperty -LiteralPath $instancePath -Name Service -ErrorAction Stop).Service
    if ($currentService -ine 'WinUSB') { throw 'Driver binding changed; configuration cancelled.' }
    if (@(Get-WinUsbInstances $runtimeRoot).Count -ne 0) { throw 'Runtime device binding changed; configuration cancelled.' }
    if (-not (Test-Path -LiteralPath $parameters)) {
        [void](New-Item -Path $parameters -ItemType Directory)
    }
    if ($configured) {
        Write-Output 'The loader interface GUID is already configured; no registry write was needed.'
    } else {
        $updated = [string[]](@($existing) + $interfaceGuid)
        if ($null -eq $property) {
            [void](New-ItemProperty -LiteralPath $parameters -Name DeviceInterfaceGUIDs -PropertyType MultiString -Value $updated)
        } else {
            Set-ItemProperty -LiteralPath $parameters -Name DeviceInterfaceGUIDs -Value $updated
        }
    }
    $verify = @((Get-ItemProperty -LiteralPath $parameters -Name DeviceInterfaceGUIDs -ErrorAction Stop).DeviceInterfaceGUIDs)
    if (@($verify | Where-Object { $_ -ieq $interfaceGuid }).Count -ne 1) {
        throw 'Could not verify the configured loader interface GUID.'
    }
    Write-Output 'Loader interface configured. Disconnect and reconnect the cold camera, then run the firmware loader.'
    exit 0
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
