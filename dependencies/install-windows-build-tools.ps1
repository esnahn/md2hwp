#requires -Version 7.4
# SPDX-License-Identifier: AGPL-3.0-only
[CmdletBinding(SupportsShouldProcess)]
param(
    [switch]$Check,
    [string]$DependencyLockPath = (Join-Path $PSScriptRoot 'lock.json'),
    [string]$DownloadRoot = (Join-Path $PSScriptRoot '../.local/dependencies/windows-build-tools')
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or
    [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne [Runtime.InteropServices.Architecture]::X64 -or
    [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne [Runtime.InteropServices.Architecture]::X64) {
    throw 'Run this script with 64-bit PowerShell 7.4 or later on Windows x64.'
}
Import-Module (Join-Path $PSScriptRoot 'windows-build-tools.psm1') -Force
$pins = Read-BuildToolsLock -Path $DependencyLockPath
$installerRoot = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer'
$vswhere = Join-Path $installerRoot 'vswhere.exe'
$installer = Join-Path $installerRoot 'setup.exe'
$sdkKey = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots' -ErrorAction SilentlyContinue
$sdkRoot = if ($sdkKey -and $sdkKey.PSObject.Properties['KitsRoot10']) { $sdkKey.KitsRoot10 }
    else { Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10' }
function Get-InstalledInstances {
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) { return @() }
    $json = & $vswhere -products '*' -format json -utf8
    if ($LASTEXITCODE -ne 0) { throw "vswhere failed (exit $LASTEXITCODE)." }
    return @($json | ConvertFrom-Json)
}
$instances = @(Get-InstalledInstances)
$status = Get-BuildToolsStatus -Instances $instances -Pins $pins -SdkRoot $sdkRoot
if ($status.Valid) {
    [pscustomobject]@{ Action = 'AlreadyValid'; Status = $status }
    return
}
if ($Check) {
    $status
    throw 'Required Windows build tools are missing or incomplete. Run this script without -Check to install them.'
}
$plan = New-BuildToolsPlan -Instances $instances -Pins $pins -InstallerPath $installer
if (-not $PSCmdlet.ShouldProcess('System Visual Studio Build Tools / MSVC / Windows SDK',
    "$($plan.Action): $($plan.Executable) $($plan.Arguments -join ' ')")) {
    $plan
    return
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Open PowerShell as Administrator, then run this script again. No installer was started.'
}
if ($plan.Action -eq 'Install') {
    $downloadDirectory = [IO.Path]::GetFullPath($DownloadRoot)
    $null = New-Item -ItemType Directory -Path $downloadDirectory -Force
    # Always fetch the current official bootstrapper; the lock version is a reference, not a digest pin.
    $temporary = Join-Path $downloadDirectory ('.download-' + [guid]::NewGuid().ToString('N') + '.exe')
    try {
        Invoke-WebRequest -Uri $pins['visual-studio-build-tools'].url -OutFile $temporary
        Assert-MicrosoftInstaller -Signature (Get-AuthenticodeSignature -LiteralPath $temporary)
        $executable = Join-Path $downloadDirectory 'vs_buildtools.exe'
        Move-Item -LiteralPath $temporary -Destination $executable -Force
    } finally {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force }
    }
} else {
    if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw "Missing Visual Studio installer: $installer" }
    Assert-MicrosoftInstaller -Signature (Get-AuthenticodeSignature -LiteralPath $installer)
    $executable = $installer
}
# Start-Process joins arguments; quote paths containing spaces explicitly. No shell execution.
$quoted = @($plan.Arguments | ForEach-Object {
    if ($_ -match '["\r\n]') { throw 'Unexpected quote/control character in installer argument.' }
    if ($_ -match '\s') { '"' + $_ + '"' } else { $_ }
})
$process = Start-Process -FilePath $executable -ArgumentList $quoted -WorkingDirectory ([IO.Path]::GetTempPath()) -WindowStyle Hidden -Wait -PassThru
$outcome = Get-BuildToolsExitStatus -Code $process.ExitCode
if ($outcome -eq 'RebootRequired') {
    [pscustomobject]@{ Action = 'RebootRequired'; InstallerExitCode = 3010 }
    Write-Warning 'Installation succeeded but requires a reboot. Reboot and run -Check before building.'
    exit 3010
}
$status = Get-BuildToolsStatus -Instances @(Get-InstalledInstances) -Pins $pins -SdkRoot $sdkRoot
if (-not $status.Valid) { throw 'Installer returned success, but required build tools are still missing. Check installer logs and rerun -Check.' }
[pscustomobject]@{ Action = $plan.Action; InstallerExitCode = 0; Status = $status }
