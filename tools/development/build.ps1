#requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Release',
    [switch]$Package
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
if ($Package -and $Configuration -ne 'Release') { throw 'Deployment archives require Release configuration.' }
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
    Copy-Item -LiteralPath (Join-Path $root 'templates/template.hwp') -Destination $templateDestination -Force
    $manifest = [IO.File]::ReadAllText((Join-Path $root 'Cargo.toml'))
    $versionMatch = [regex]::Match($manifest, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"\s*$')
    if (-not $versionMatch.Success) { throw 'Cannot determine package version from Cargo.toml.' }
    $version = $versionMatch.Groups[1].Value
    $readme = [IO.File]::ReadAllText((Join-Path $root 'README.md'))
    # Deployment ZIPs do not contain the repository's docs and examples folders.
    # Keep their links usable from the README shipped beside the executables.
    $readme = [regex]::Replace($readme, '\]\((?<path>(?:docs|examples)/[^)\r\n]+)\)', {
        param($match)
        '](https://github.com/esnahn/md2hwp/blob/v' + $version + '/' + $match.Groups['path'].Value + ')'
    })
    [IO.File]::WriteAllText((Join-Path $destination 'README.md'), $readme, [Text.UTF8Encoding]::new($false))
    Write-Output "Deployment files: $destination (md2hwp.exe, md2hwp-backend.exe, template.hwp, README.md)"
    if ($Package) {
        $dist = [IO.Path]::GetFullPath((Join-Path $root 'target/dist'))
        $null = New-Item -ItemType Directory -Path $dist -Force
        $stage = Join-Path $dist ('.package-' + [Guid]::NewGuid().ToString('N'))
        $null = New-Item -ItemType Directory -Path $stage
        try {
            foreach ($name in @('md2hwp.exe', 'md2hwp-backend.exe', 'README.md')) {
                Copy-Item -LiteralPath (Join-Path $destination $name) -Destination $stage
            }
            # Ship the tracked default; never adopt a user's deployment template implicitly.
            Copy-Item -LiteralPath (Join-Path $root 'templates/template.hwp') -Destination $stage
            $archive = Join-Path $dist "md2hwp-v$version-windows-x64.zip"
            $temporaryArchive = Join-Path $stage 'package.zip'
            $files = @('md2hwp.exe', 'md2hwp-backend.exe', 'template.hwp', 'README.md') | ForEach-Object { Join-Path $stage $_ }
            Compress-Archive -LiteralPath $files -DestinationPath $temporaryArchive
            Move-Item -LiteralPath $temporaryArchive -Destination $archive -Force
            Write-Output "Package: $archive"
            Write-Output ('SHA-256: ' + (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant())
        } finally {
            $resolvedStage = [IO.Path]::GetFullPath($stage)
            if ([IO.Path]::GetDirectoryName($resolvedStage) -ne $dist -or [IO.Path]::GetFileName($resolvedStage) -notmatch '^\.package-[0-9a-f]{32}$') {
                throw 'Refusing cleanup outside the package staging directory.'
            }
            Remove-Item -LiteralPath $resolvedStage -Recurse -Force
        }
    }
} finally { Pop-Location }
