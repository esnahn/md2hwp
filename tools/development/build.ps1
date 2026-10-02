#requires -Version 7.4
[CmdletBinding()]
param([ValidateSet('Debug', 'Release')][string]$Configuration = 'Release')
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$project = Join-Path $root 'backends/hancom-automation/Md2Hwp.Backend.csproj'
$destination = Join-Path $root "target/$($Configuration.ToLowerInvariant())"
$published = Join-Path ([IO.Path]::GetDirectoryName($project)) "bin/$Configuration/net10.0-windows/win-x64/publish"
Push-Location $root
try {
    $cargoArgs = @('build', '-p', 'md2hwp', '--target-dir', (Join-Path $root 'target'))
    if ($Configuration -eq 'Release') { $cargoArgs += '--release' }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed.' }
    & (Join-Path $PSHOME 'pwsh.exe') -NoProfile -File (Join-Path $PSScriptRoot 'dotnet.ps1') publish $project --configuration $Configuration -p:PublishProfile=FrameworkDependent
    if ($LASTEXITCODE -ne 0) { throw 'Backend publish failed.' }
    Copy-Item -LiteralPath (Join-Path $published 'md2hwp-backend.exe') -Destination (Join-Path $destination 'md2hwp-backend.exe') -Force
    $templateDestination = Join-Path $destination 'template.hwp'
    if (-not (Test-Path -LiteralPath $templateDestination)) {
        Copy-Item -LiteralPath (Join-Path $published 'template.hwp') -Destination $templateDestination
    }
    Write-Output "Deployment files: $destination (md2hwp.exe, md2hwp-backend.exe, template.hwp)"
} finally { Pop-Location }
