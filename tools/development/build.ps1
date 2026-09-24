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
    foreach ($name in @('md2hwp-backend.exe', 'template.hwp')) {
        Copy-Item -LiteralPath (Join-Path $published $name) -Destination (Join-Path $destination $name) -Force
    }
    Write-Output "Deployment files: $destination (md2hwp.exe, md2hwp-backend.exe, template.hwp)"
} finally { Pop-Location }
