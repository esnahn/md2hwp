#requires -Version 5.1
# Offline fallback and rollback checks; network calls are replaced in this scope.
param([ValidateSet('fallback','integrity')][string]$Case = 'fallback', [string]$InstallRoot)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$lock = Get-Content -LiteralPath (Join-Path $root 'dependencies/lock.json') -Raw | ConvertFrom-Json
$pin = @($lock.dependencies | Where-Object name -eq 'pandoc')[0].pins[0]
$archiveFixture = Join-Path $root ".local/dependencies/pandoc/$($pin.version)/pandoc-$($pin.version)-windows-x86_64.zip"
if (-not $InstallRoot) { $InstallRoot = Join-Path $root ('artifacts/pandoc-setup-smoke-' + [guid]::NewGuid().ToString('N')) }
$env:MD2HWP_PANDOC_VERSION = $pin.version
$env:MD2HWP_PANDOC_URL = $pin.url
$env:MD2HWP_PANDOC_SHA256 = if ($Case -eq 'integrity') { '0' * 64 } else { $pin.sha256 }
$global:md2hwpTestDownloadCount = 0
function Invoke-WebRequest {
    param($Uri, $OutFile, [switch]$UseBasicParsing, $TimeoutSec)
    if ($Uri -like 'https://raw.githubusercontent.com/jgm/pandoc/*/COPYRIGHT') {
        [IO.File]::WriteAllText($OutFile, 'Synthetic test notice; real setup downloads the upstream notice.')
        return
    }
    $global:md2hwpTestDownloadCount++
    if ($Case -eq 'fallback' -and $global:md2hwpTestDownloadCount -eq 1) { throw 'Simulated unavailable preferred release.' }
    if ($Uri -cne $pin.url) { throw "Unexpected network target: $Uri" }
    Copy-Item -LiteralPath $archiveFixture -Destination $OutFile
}
function Invoke-RestMethod {
    param($Uri, $Headers, $TimeoutSec)
    if ($Uri -cne 'https://api.github.com/repos/jgm/pandoc/releases/latest') { throw 'Unexpected API target.' }
    [pscustomobject]@{ tag_name=$pin.version; draft=$false; prerelease=$false; assets=@(
        [pscustomobject]@{name="pandoc-$($pin.version)-windows-x86_64.zip"; browser_download_url=$pin.url; digest=('sha256:' + $pin.sha256)}
    ) }
}
& (Join-Path $root 'apps/md2hwp/setup-pandoc.ps1') -InstallRoot $InstallRoot
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ($Case -eq 'integrity') { throw 'Integrity failure unexpectedly returned success.' }
$pointer = [IO.File]::ReadAllText((Join-Path $InstallRoot 'current.txt'))
if (-not [IO.File]::Exists((Join-Path $InstallRoot $pointer))) { throw 'Installed executable missing.' }
$installation = @(Get-ChildItem -LiteralPath $InstallRoot -Directory)[0].FullName
foreach ($name in @('upstream.zip','COPYRIGHT','UPSTREAM.txt')) {
    if (-not [IO.File]::Exists((Join-Path $installation $name))) { throw "Missing preserved notice/archive: $name" }
}
if ($global:md2hwpTestDownloadCount -ne 2) { throw 'Preferred-first fallback was not exercised.' }
Write-Output "Offline fallback passed; retained artifacts: $InstallRoot"
