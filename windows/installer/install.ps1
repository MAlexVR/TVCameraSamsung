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
                throw 'Reparse points are not accepted in package or installation paths.'
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
        Write-Output 'Usage: powershell -NoProfile -File installer\install.ps1 -Check'
        Write-Output 'To install this unsigned diagnostic for the current user, use -Install.'
        Write-Output 'No camera driver, firmware loader or security changes are included.'
        exit 0
    }
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or -not [Environment]::Is64BitOperatingSystem) {
        throw 'This package requires 64-bit Windows.'
    }
    $version = '1.0.0'
    $package = Split-Path -Parent $PSScriptRoot
    Assert-PlainPath $package
    $entries = @(
        @{ Source = (Join-Path $package 'samsung-camera-windows.exe'); Name = 'samsung-camera-windows.exe' },
        @{ Source = (Join-Path $package 'RUN-DIAGNOSTICS.cmd'); Name = 'RUN-DIAGNOSTICS.cmd' },
        @{ Source = (Join-Path $package 'PACKAGE_STATUS.json'); Name = 'PACKAGE_STATUS.json' },
        @{ Source = (Join-Path $PSScriptRoot 'README.md'); Name = 'README.md' }
    )
    foreach ($entry in $entries) {
        Assert-PlainPath $entry.Source
        if (-not (Test-Path -LiteralPath $entry.Source -PathType Leaf)) {
            throw ('Missing package file: ' + $entry.Name)
        }
    }
    $status = Get-Content -LiteralPath (Join-Path $package 'PACKAGE_STATUS.json') -Raw | ConvertFrom-Json
    if ($status.version -ne $version -or $status.target -ne 'x86_64-pc-windows-gnu' -or
        $status.read_only -isnot [bool] -or -not $status.read_only -or
        $status.firmware_loader_implemented -isnot [bool] -or $status.firmware_loader_implemented -or
        $status.driver_package_included -isnot [bool] -or $status.driver_package_included -or
        $status.windows_executable_included -isnot [bool] -or -not $status.windows_executable_included -or
        $status.signature -ne 'unsigned') {
        throw 'Package status must describe the unsigned read-only 1.0.0 diagnostic, not a driver.'
    }
    if ($status.binary_sha256 -isnot [string] -or $status.binary_sha256 -notmatch '\A[0-9a-fA-F]{64}\z') {
        throw 'Package status has no valid binary SHA256.'
    }
    $expectedHash = $status.binary_sha256
    $actualHash = (Get-FileHash -LiteralPath $entries[0].Source -Algorithm SHA256).Hash
    if ($actualHash -ne $expectedHash) { throw 'Executable SHA256 mismatch; nothing installed.' }
    if (-not $env:LOCALAPPDATA -or -not (Test-Path -LiteralPath $env:LOCALAPPDATA -PathType Container)) {
        throw 'A normal Windows user profile with LocalAppData is required.'
    }
    Assert-PlainPath $env:LOCALAPPDATA
    $base = Join-Path $env:LOCALAPPDATA 'SamsungCamera'
    $target = Join-Path $base $version
    Assert-PlainPath $base
    Assert-PlainPath $target
    if (Test-Path -LiteralPath $target) { throw 'Target already exists; no files will be overwritten.' }
    if ((Test-Path -LiteralPath $base) -and -not (Test-Path -LiteralPath $base -PathType Container)) {
        throw 'Installation parent is not a directory.'
    }
    if (-not $Install) {
        Write-Output ('Package checked. No writes performed. Target: ' + $target)
        exit 0
    }
    if ($ConfirmInstall) {
        Write-Output 'This installs an unsigned read-only diagnostic, NOT a camera driver or firmware loader.'
        $answer = Read-Host 'Type INSTALL to continue, or press Enter to cancel'
        if ($answer -cne 'INSTALL') { Write-Output 'Cancelled; nothing installed.'; exit 0 }
    }
    # Only explicit -Install reaches filesystem mutation; never run the EXE here.
    [void][System.IO.Directory]::CreateDirectory($base)
    Assert-PlainPath $base
    $staging = Join-Path $base ('.install-' + [Guid]::NewGuid().ToString('N'))
    if (Test-Path -LiteralPath $staging) { throw 'Staging path already exists.' }
    [void][System.IO.Directory]::CreateDirectory($staging)
    $createdFiles = @()
    try {
        Assert-PlainPath $staging
        foreach ($entry in $entries) {
            Assert-PlainPath $entry.Source
            $destination = Join-Path $staging $entry.Name
            [System.IO.File]::Copy($entry.Source, $destination, $false)
            $createdFiles += $destination
        }
        $copiedHash = (Get-FileHash -LiteralPath (Join-Path $staging 'samsung-camera-windows.exe') -Algorithm SHA256).Hash
        if ($copiedHash -ne $expectedHash) { throw 'Copied executable SHA256 mismatch.' }
        Assert-PlainPath $base
        Assert-PlainPath $target
        # Directory.Move refuses an existing target, including a concurrent install.
        [System.IO.Directory]::Move($staging, $target)
    } catch {
        $installFailure = $_
        # Never recurse or touch a published/existing installation during cleanup.
        try {
            Assert-PlainPath $staging
            foreach ($file in $createdFiles) {
                Assert-PlainPath $file
                [System.IO.File]::Delete($file)
            }
            [System.IO.Directory]::Delete($staging, $false)
        } catch {
            Write-Warning 'Temporary installation cleanup was incomplete; inspect the SamsungCamera directory.'
        }
        throw $installFailure
    }
    Write-Output ('Installed diagnostic files only: ' + $target)
    Write-Output 'Run RUN-DIAGNOSTICS.cmd there manually when ready. No camera access occurred during installation.'
    exit 0
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
