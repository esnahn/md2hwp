#requires -Version 7.0

[CmdletBinding()]
param(
    [string]$DependencyLockPath,
    [string]$InstallRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$stateRoot = Join-Path $repositoryRoot ".local\state\dotnet-installer"
$env:DOTNET_CLI_HOME = Join-Path $stateRoot "cli-home"
$env:APPDATA = Join-Path $stateRoot "appdata"
$env:DOTNET_CLI_TELEMETRY_OPTOUT = "1"
$env:DOTNET_ADD_GLOBAL_TOOLS_TO_PATH = "0"
$env:DOTNET_NOLOGO = "1"
$env:DOTNET_SKIP_FIRST_TIME_EXPERIENCE = "1"
if ([string]::IsNullOrWhiteSpace($DependencyLockPath)) {
    $DependencyLockPath = Join-Path $PSScriptRoot "lock.json"
}
if ([string]::IsNullOrWhiteSpace($InstallRoot)) {
    $InstallRoot = Join-Path $repositoryRoot ".local\dependencies\dotnet"
}

$lock = Get-Content -Raw -Encoding UTF8 -LiteralPath ([IO.Path]::GetFullPath($DependencyLockPath)) |
    ConvertFrom-Json
$dependencies = @($lock.dependencies | Where-Object { $_.name -eq "dotnet-sdk" })
if ($dependencies.Count -ne 1) {
    throw "Expected exactly one dependency named dotnet-sdk."
}
$pins = @($dependencies[0].pins | Where-Object {
        $_.name -eq "dotnet-sdk-windows-x86-64" -and $_.kind -eq "release-artifact"
    })
if ($pins.Count -ne 1) {
    throw "Expected exactly one dotnet-sdk-windows-x86-64 release artifact."
}

$pin = $pins[0]
$version = [string]$pin.version
$expectedHash = [string]$pin.sha512
if ($expectedHash -cnotmatch "^[0-9A-F]{128}$") {
    throw "The .NET SDK lock contains an invalid SHA-512 value."
}

$absoluteInstallRoot = [IO.Path]::GetFullPath($InstallRoot)
$installDirectory = [IO.Path]::GetFullPath((Join-Path $absoluteInstallRoot $version))
if (-not $installDirectory.StartsWith(
        $absoluteInstallRoot + [IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase
    )) {
    throw ".NET SDK install directory escaped its declared root: $installDirectory"
}
$dotnetPath = Join-Path $installDirectory "dotnet.exe"
$archiveName = "dotnet-sdk-$version-win-x64.zip"
$archivePath = Join-Path $installDirectory $archiveName

function Test-InstalledSdk {
    if (-not (Test-Path -LiteralPath $dotnetPath -PathType Leaf) -or
        -not (Test-Path -LiteralPath $archivePath -PathType Leaf) -or
        (Get-FileHash -LiteralPath $archivePath -Algorithm SHA512).Hash -cne $expectedHash -or
        -not (Test-Path -LiteralPath (Join-Path $installDirectory "sdk\$version") -PathType Container)) {
        return $false
    }
    $actualVersion = & $dotnetPath --version
    return $LASTEXITCODE -eq 0 -and $actualVersion -ceq $version
}

if (Test-InstalledSdk) {
    [pscustomobject]@{
        Action = "AlreadyValid"
        Version = $version
        DotNet = $dotnetPath
        Archive = $archivePath
    }
    return
}

$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) ("md2hwp-dotnet-" + [guid]::NewGuid().ToString("N"))
$temporaryArchive = Join-Path $temporaryDirectory $archiveName
$stagedDirectory = Join-Path $temporaryDirectory "staged"

try {
    $null = New-Item -ItemType Directory -Path $stagedDirectory -Force
    Invoke-WebRequest -Uri ([uri]$pin.url) -OutFile $temporaryArchive
    $actualHash = (Get-FileHash -LiteralPath $temporaryArchive -Algorithm SHA512).Hash
    if ($actualHash -cne $expectedHash) {
        throw ".NET SDK archive SHA-512 mismatch. Expected $expectedHash, got $actualHash."
    }

    Expand-Archive -LiteralPath $temporaryArchive -DestinationPath $stagedDirectory
    $stagedDotnet = Join-Path $stagedDirectory "dotnet.exe"
    if (-not (Test-Path -LiteralPath $stagedDotnet -PathType Leaf) -or
        -not (Test-Path -LiteralPath (Join-Path $stagedDirectory "sdk\$version") -PathType Container)) {
        throw "The official archive did not contain the expected .NET SDK layout."
    }
    $actualVersion = & $stagedDotnet --version
    if ($LASTEXITCODE -ne 0 -or $actualVersion -cne $version) {
        throw "Downloaded .NET SDK version mismatch. Expected '$version', got '$actualVersion'."
    }
    Copy-Item -LiteralPath $temporaryArchive -Destination (Join-Path $stagedDirectory $archiveName)

    $null = New-Item -ItemType Directory -Path $absoluteInstallRoot -Force
    if (Test-Path -LiteralPath $installDirectory) {
        Remove-Item -LiteralPath $installDirectory -Recurse -Force
    }
    Move-Item -LiteralPath $stagedDirectory -Destination $installDirectory

    [pscustomobject]@{
        Action = "Installed"
        Version = $version
        DotNet = $dotnetPath
        Archive = $archivePath
    }
}
finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}
