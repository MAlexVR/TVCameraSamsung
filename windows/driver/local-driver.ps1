# SPDX-License-Identifier: GPL-3.0-only
# Local-only WinUSB binding for the cold camera 04e8:205c on THIS computer.
# Signs the project INF catalog with a self-generated certificate that is trusted
# machine-wide until -Remove runs. Secure Boot and signature enforcement stay on.
# Requires the Windows SDK (makecat.exe, signtool.exe).
[CmdletBinding()]
param(
    [switch]$BuildOnly,
    [switch]$Install,
    [switch]$Remove,
    [switch]$Check
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$subject = 'CN=TVCameraSamsung Local Driver (this computer only)'
$work = Join-Path $env:LOCALAPPDATA 'SamsungCamera\local-driver'
$infName = 'samsung-vg-stc3000.inf'
$catName = 'samsung-vg-stc3000.cat'

function Find-SdkTool([string]$Name) {
    $root = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $hit = @(Get-ChildItem -LiteralPath $root -Recurse -Filter $Name -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending)
    if ($hit.Count -eq 0) { throw ($Name + ' not found. Install the Windows SDK (Signing Tools).') }
    return $hit[0].FullName
}

function Get-LocalCert {
    @(Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $subject -and $_.NotAfter -gt (Get-Date) } |
        Sort-Object NotAfter -Descending) | Select-Object -First 1
}

function Remove-LocalCerts {
    foreach ($store in 'Cert:\LocalMachine\Root', 'Cert:\LocalMachine\TrustedPublisher', 'Cert:\CurrentUser\My') {
        Get-ChildItem $store | Where-Object { $_.Subject -eq $subject } | ForEach-Object {
            Remove-Item -LiteralPath $_.PSPath -DeleteKey -ErrorAction SilentlyContinue
        }
    }
}

function Test-Admin {
    ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Build-Package {
    $sourceInf = Join-Path $PSScriptRoot $infName
    $definition = Get-Content -LiteralPath $sourceInf -Raw
    if ($definition -match '(?i)PID_205D') { throw 'The INF must never match initialized device 04e8:205d.' }
    if (-not $definition.Contains('USB\VID_04E8&PID_205C')) { throw 'INF does not match the boot device.' }
    [void][System.IO.Directory]::CreateDirectory($work)
    $cert = Get-LocalCert
    if ($null -eq $cert) {
        $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $subject -KeyExportPolicy NonExportable `
            -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 -NotAfter (Get-Date).AddMonths(12) `
            -CertStoreLocation Cert:\CurrentUser\My
    }
    Copy-Item -LiteralPath $sourceInf -Destination (Join-Path $work $infName) -Force
    $cdf = Join-Path $work 'samsung-vg-stc3000.cdf'
    Set-Content -LiteralPath $cdf -Encoding ASCII -Value @(
        '[CatalogHeader]', ('Name=' + $catName), 'PublicVersion=0x0000001', 'EncodingType=0x00010001',
        'CATATTR1=0x10010001:OSAttr:2:10.0', '[CatalogFiles]', ('<HASH>' + $infName + '=' + $infName),
        ('<HASH>' + $infName + 'ATTR1=0x10010001:OSAttr:2:10.0'))
    Push-Location $work
    try {
        if (Test-Path -LiteralPath $catName) { Remove-Item -LiteralPath $catName -Force }
        & (Find-SdkTool 'makecat.exe') -v $cdf | Out-Null
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $catName)) { throw 'makecat failed.' }
        & (Find-SdkTool 'signtool.exe') sign /fd SHA256 /sha1 $cert.Thumbprint /s My $catName | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'signtool sign failed.' }
    } finally { Pop-Location }
    return $cert
}

try {
    if (@($BuildOnly, $Install, $Remove, $Check | Where-Object { $_ }).Count -ne 1) {
        Write-Output 'Usage: local-driver.ps1 -Check | -BuildOnly | -Install (admin) | -Remove (admin)'
        exit 0
    }
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or -not [Environment]::Is64BitOperatingSystem) {
        throw 'This utility requires 64-bit Windows 10 or later.'
    }
    if ($Check) {
        $cert = Get-LocalCert
        Write-Output ('Local certificate present: ' + ($null -ne $cert))
        Write-Output ('Trusted machine-wide: ' + (@(Get-ChildItem Cert:\LocalMachine\Root | Where-Object { $_.Subject -eq $subject }).Count -gt 0))
        Write-Output ('Driver package staged: ' + (Test-Path -LiteralPath (Join-Path $work $catName)))
        exit 0
    }
    if ($BuildOnly) {
        $cert = Build-Package
        Write-Output ('Signed catalog built: ' + (Join-Path $work $catName))
        Write-Output ('Certificate thumbprint: ' + $cert.Thumbprint + ' (not trusted yet; nothing installed)')
        exit 0
    }
    if (-not (Test-Admin)) { throw 'Administrator privileges are required (UAC).' }
    $pnputil = Join-Path $env:SystemRoot 'System32\pnputil.exe'
    if ($Remove) {
        Write-Output 'Removing the local driver package and trust for this computer.'
        $listing = @(& $pnputil /enum-drivers 2>&1 | ForEach-Object { "$_" })
        $published = $null
        for ($i = 0; $i -lt $listing.Count; $i++) {
            if ($listing[$i] -match '(oem\d+\.inf)\s*$' -and ($listing[$i..([Math]::Min($i + 6, $listing.Count - 1))] -join ' ') -match 'samsung-vg-stc3000') {
                $published = $Matches[1]; break
            }
        }
        if ($published) { & $pnputil /delete-driver $published /uninstall /force | ForEach-Object { Write-Output $_ } }
        Remove-LocalCerts
        Write-Output 'Done. Reconnect the camera; it returns to the standard Windows USB driver.'
        exit 0
    }
    Write-Output 'This trusts a certificate generated on this computer, for this driver package only, and binds'
    Write-Output 'Microsoft WinUSB to Samsung 04e8:205c. Initialized 04e8:205d is never matched.'
    Write-Output 'Undo it at any time with LOCAL-DRIVER-REMOVE.cmd (local-driver.ps1 -Remove).'
    $answer = Read-Host 'Type INSTALL LOCAL to continue, or press Enter to cancel'
    if ($answer -cne 'INSTALL LOCAL') { Write-Output 'Cancelled; nothing changed.'; exit 0 }
    $cert = Build-Package
    $cer = Join-Path $work 'local-driver.cer'
    [void](Export-Certificate -Cert $cert -FilePath $cer -Force)
    foreach ($store in 'Cert:\LocalMachine\Root', 'Cert:\LocalMachine\TrustedPublisher') {
        [void](Import-Certificate -FilePath $cer -CertStoreLocation $store)
    }
    $output = & $pnputil /add-driver (Join-Path $work $infName) /install 2>&1
    $result = $LASTEXITCODE
    $output | ForEach-Object { Write-Output $_ }
    if ($result -ne 0 -and $result -ne 3010) { throw ('PnPUtil failed with exit code ' + $result + '. Run LOCAL-DRIVER-REMOVE.cmd to undo the trust.') }
    Write-Output 'Installed. Disconnect and reconnect the camera, then run TEST-CAMERA.cmd.'
    exit 0
} catch {
    Write-Error $_.Exception.Message
    exit 1
}
