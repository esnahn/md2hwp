#requires -Version 7.0

[CmdletBinding()]
param(
    [string]$DependencyLockPath,
    [string]$InstallRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if ([string]::IsNullOrWhiteSpace($DependencyLockPath)) {
    $DependencyLockPath = Join-Path $PSScriptRoot "lock.json"
}
if ([string]::IsNullOrWhiteSpace($InstallRoot)) {
    $InstallRoot = Join-Path $repositoryRoot ".local\dependencies\pandoc"
}

$lock = Get-Content -Raw -Encoding UTF8 -LiteralPath ([IO.Path]::GetFullPath($DependencyLockPath)) |
    ConvertFrom-Json
$dependencies = @($lock.dependencies | Where-Object { $_.name -eq "pandoc" })
if ($dependencies.Count -ne 1) {
    throw "Expected exactly one dependency named pandoc."
}
$pins = @($dependencies[0].pins | Where-Object {
        $_.name -eq "pandoc-windows-x86-64" -and $_.kind -eq "release-artifact"
    })
if ($pins.Count -ne 1) {
    throw "Expected exactly one pandoc-windows-x86-64 release artifact."
}

$pin = $pins[0]
$version = [string]$pin.version
$expectedHash = [string]$pin.sha256
if ($expectedHash -cnotmatch "^[0-9A-F]{64}$") {
    throw "The Pandoc lock contains an invalid SHA-256 value."
}

$absoluteInstallRoot = [IO.Path]::GetFullPath($InstallRoot)
$installDirectory = [IO.Path]::GetFullPath((Join-Path $absoluteInstallRoot $version))
if (-not $installDirectory.StartsWith($absoluteInstallRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Pandoc install directory escaped its declared root: $installDirectory"
}
$pandocPath = Join-Path $installDirectory "pandoc.exe"
$archivePath = Join-Path $installDirectory "pandoc-$version-windows-x86_64.zip"

if ((Test-Path -LiteralPath $pandocPath -PathType Leaf) -and
    (Test-Path -LiteralPath $archivePath -PathType Leaf) -and
    (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash -ceq $expectedHash) {
    $versionLine = & $pandocPath --version | Select-Object -First 1
    if ($versionLine -ceq "pandoc $version") {
        [pscustomobject]@{ Action = "AlreadyValid"; Version = $version; Pandoc = $pandocPath; Archive = $archivePath }
        return
    }
}

$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) ("md2hwp-pandoc-" + [guid]::NewGuid().ToString("N"))
$temporaryArchive = Join-Path $temporaryDirectory "pandoc.zip"
$extractDirectory = Join-Path $temporaryDirectory "extracted"
$stagedDirectory = Join-Path $temporaryDirectory "staged"

try {
    $null = New-Item -ItemType Directory -Path $extractDirectory -Force
    Invoke-WebRequest -Uri ([uri]$pin.url) -OutFile $temporaryArchive
    $actualHash = (Get-FileHash -LiteralPath $temporaryArchive -Algorithm SHA256).Hash
    if ($actualHash -cne $expectedHash) {
        throw "Pandoc archive SHA-256 mismatch. Expected $expectedHash, got $actualHash."
    }
    Expand-Archive -LiteralPath $temporaryArchive -DestinationPath $extractDirectory
    $executables = @(Get-ChildItem -LiteralPath $extractDirectory -Recurse -File -Filter "pandoc.exe")
    if ($executables.Count -ne 1) {
        throw "Expected exactly one pandoc.exe in the official archive; found $($executables.Count)."
    }
    $versionLine = & $executables[0].FullName --version | Select-Object -First 1
    if ($versionLine -cne "pandoc $version") {
        throw "Downloaded Pandoc version mismatch. Expected 'pandoc $version', got '$versionLine'."
    }

    $null = New-Item -ItemType Directory -Path $stagedDirectory
    Copy-Item -LiteralPath $executables[0].FullName -Destination (Join-Path $stagedDirectory "pandoc.exe")
    Copy-Item -LiteralPath $temporaryArchive -Destination (Join-Path $stagedDirectory "pandoc-$version-windows-x86_64.zip")
    $null = New-Item -ItemType Directory -Path $absoluteInstallRoot -Force
    if (Test-Path -LiteralPath $installDirectory) {
        Remove-Item -LiteralPath $installDirectory -Recurse -Force
    }
    Move-Item -LiteralPath $stagedDirectory -Destination $installDirectory

    [pscustomobject]@{ Action = "Installed"; Version = $version; Pandoc = $pandocPath; Archive = $archivePath }
}
finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}
