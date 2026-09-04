#requires -Version 7.0

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$version = "10.0.400"
$dotnetRoot = Join-Path $repositoryRoot ".local\dependencies\dotnet\$version"
$dotnetPath = Join-Path $dotnetRoot "dotnet.exe"
if (-not (Test-Path -LiteralPath $dotnetPath -PathType Leaf)) {
    throw "Missing repository-local .NET SDK. Run .\dependencies\install-dotnet-sdk.ps1 first."
}

$stateRoot = Join-Path $repositoryRoot ".local\state\dotnet"
$env:DOTNET_ROOT = $dotnetRoot
$env:DOTNET_CLI_HOME = Join-Path $stateRoot "cli-home"
$env:NUGET_PACKAGES = Join-Path $stateRoot "nuget-packages"
$env:APPDATA = Join-Path $stateRoot "appdata"
$env:DOTNET_CLI_TELEMETRY_OPTOUT = "1"
$env:DOTNET_ADD_GLOBAL_TOOLS_TO_PATH = "0"
$env:DOTNET_NOLOGO = "1"
$env:DOTNET_SKIP_FIRST_TIME_EXPERIENCE = "1"

& $dotnetPath @args
exit $LASTEXITCODE
