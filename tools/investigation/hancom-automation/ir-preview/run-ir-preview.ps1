#requires -Version 5.1

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("probe", "render", "prepare-template-pair", "export-images", "investigate-ranges", "author-tagged", "render-tagged")]
    [string]$Mode,

    [string]$Template,

    [string]$Baseline,

    [string]$Marked,

    [string]$Document,

    [string]$Ir,

    [Alias("Profile")]
    [string]$ProfilePath,

    [string]$Output,

    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release",

    [switch]$Visible,

    [string]$RustLauncher,

    [string]$RuntimeHostPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$requiredHostProcess = 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
$actualHostProcess = [IO.Path]::GetFullPath(
    [Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
)
if ($PSVersionTable.PSEdition -ne "Desktop" -or
    $PSVersionTable.PSVersion.Major -ne 5 -or
    $PSVersionTable.PSVersion.Minor -ne 1 -or
    -not [Environment]::Is64BitProcess -or
    [Threading.Thread]::CurrentThread.ApartmentState -ne [Threading.ApartmentState]::STA -or
    -not [Environment]::UserInteractive -or
    [Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0 -or
    -not $actualHostProcess.Equals($requiredHostProcess, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Run the C# Hancom preview from interactive Windows PowerShell 5.1 x64 STA."
}

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\..\..\.."))
$dotnetRoot = Join-Path $repositoryRoot ".local\dependencies\dotnet\10.0.400"
$dotnetPath = Join-Path $dotnetRoot "dotnet.exe"
$assemblyPath = Join-Path $PSScriptRoot "bin\$Configuration\net10.0-windows\md2hwp-backend.dll"
$requiredPaths = @()
if ($RustLauncher) {
    if ($Mode -ne 'render-tagged' -or $Visible) {
        throw 'RustLauncher supports hidden render-tagged only.'
    }
    $workerExecutable = Join-Path $PSScriptRoot "bin\$Configuration\net10.0-windows\win-x64\publish\md2hwp-backend.exe"
    $requiredPaths += @($RustLauncher, $workerExecutable)
} else {
    if ($RuntimeHostPath) { throw 'RuntimeHostPath requires RustLauncher.' }
    $requiredPaths += @($dotnetPath, $assemblyPath)
}
foreach ($path in $requiredPaths) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing C# preview prerequisite: $path"
    }
}

function Resolve-InvocationPath([string]$Path) {
    if ([IO.Path]::IsPathRooted($Path)) {
        return [IO.Path]::GetFullPath($Path)
    }
    return [IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
}

$previewArguments = @($assemblyPath, $Mode)
switch ($Mode) {
    { $_ -in @("author-tagged", "render-tagged") } {
        if ([string]::IsNullOrWhiteSpace($Template) -or
            [string]::IsNullOrWhiteSpace($Output) -or
            -not [string]::IsNullOrWhiteSpace($Baseline) -or
            -not [string]::IsNullOrWhiteSpace($Marked) -or
            -not [string]::IsNullOrWhiteSpace($Document) -or
            -not [string]::IsNullOrWhiteSpace($ProfilePath) -or
            ($Mode -eq "render-tagged" -and [string]::IsNullOrWhiteSpace($Ir)) -or
            ($Mode -eq "author-tagged" -and -not [string]::IsNullOrWhiteSpace($Ir))) {
            throw "Tagged modes require -Template and -Output; only render-tagged requires -Ir. No -Profile is accepted."
        }
        $previewArguments += @("--template", (Resolve-InvocationPath $Template), "--output", (Resolve-InvocationPath $Output))
        if ($Mode -eq "render-tagged") { $previewArguments += @("--ir", (Resolve-InvocationPath $Ir)) }
    }
    "investigate-ranges" {
        if ([string]::IsNullOrWhiteSpace($Template) -or
            [string]::IsNullOrWhiteSpace($Marked) -or
            [string]::IsNullOrWhiteSpace($Output) -or
            -not [string]::IsNullOrWhiteSpace($Baseline) -or
            -not [string]::IsNullOrWhiteSpace($Document) -or
            -not [string]::IsNullOrWhiteSpace($Ir) -or
            -not [string]::IsNullOrWhiteSpace($ProfilePath)) {
            throw "Range investigation requires -Template, -Marked, and -Output."
        }
        $previewArguments += @(
            "--template", (Resolve-InvocationPath $Template),
            "--marked", (Resolve-InvocationPath $Marked),
            "--output", (Resolve-InvocationPath $Output)
        )
    }
    "probe" {
        if ([string]::IsNullOrWhiteSpace($Template) -or
            -not [string]::IsNullOrWhiteSpace($Baseline) -or
            -not [string]::IsNullOrWhiteSpace($Marked) -or
            -not [string]::IsNullOrWhiteSpace($ProfilePath) -or
            -not [string]::IsNullOrWhiteSpace($Document) -or
            -not [string]::IsNullOrWhiteSpace($Ir) -or
            -not [string]::IsNullOrWhiteSpace($Output)) {
            throw "Probe mode requires only -Template."
        }
        $previewArguments += @("--template", (Resolve-InvocationPath $Template))
    }
    "render" {
        if ([string]::IsNullOrWhiteSpace($Template) -or
            [string]::IsNullOrWhiteSpace($Ir) -or
            [string]::IsNullOrWhiteSpace($ProfilePath) -or
            [string]::IsNullOrWhiteSpace($Output) -or
            -not [string]::IsNullOrWhiteSpace($Baseline) -or
            -not [string]::IsNullOrWhiteSpace($Marked) -or
            -not [string]::IsNullOrWhiteSpace($Document)) {
            throw "Render mode requires -Template, -Ir, -Profile, and -Output."
        }
        $previewArguments += @(
            "--template", (Resolve-InvocationPath $Template),
            "--ir", (Resolve-InvocationPath $Ir),
            "--profile", (Resolve-InvocationPath $ProfilePath),
            "--output", (Resolve-InvocationPath $Output)
        )
    }
    "prepare-template-pair" {
        if ([string]::IsNullOrWhiteSpace($Template) -or
            [string]::IsNullOrWhiteSpace($Baseline) -or
            [string]::IsNullOrWhiteSpace($Marked) -or
            -not [string]::IsNullOrWhiteSpace($ProfilePath) -or
            -not [string]::IsNullOrWhiteSpace($Document) -or
            -not [string]::IsNullOrWhiteSpace($Ir) -or
            -not [string]::IsNullOrWhiteSpace($Output)) {
            throw "Prepare-comparison mode requires -Template, -Baseline, and -Marked."
        }
        $previewArguments += @(
            "--template", (Resolve-InvocationPath $Template),
            "--baseline", (Resolve-InvocationPath $Baseline),
            "--marked", (Resolve-InvocationPath $Marked)
        )
    }
    "export-images" {
        if ([string]::IsNullOrWhiteSpace($Document) -or
            [string]::IsNullOrWhiteSpace($Output) -or
            -not [string]::IsNullOrWhiteSpace($ProfilePath) -or
            -not [string]::IsNullOrWhiteSpace($Template) -or
            -not [string]::IsNullOrWhiteSpace($Baseline) -or
            -not [string]::IsNullOrWhiteSpace($Marked) -or
            -not [string]::IsNullOrWhiteSpace($Ir)) {
            throw "Export-images mode requires -Document and -Output."
        }
        $previewArguments += @(
            "--document", (Resolve-InvocationPath $Document),
            "--output", (Resolve-InvocationPath $Output)
        )
    }
}
if ($Visible) {
    $previewArguments += "--visible"
}

if ($RustLauncher) {
    $rustArguments = @('render-hwp', '--worker', $workerExecutable,
        '--ir', (Resolve-InvocationPath $Ir), '--template', (Resolve-InvocationPath $Template),
        '--output', (Resolve-InvocationPath $Output))
    if ($RuntimeHostPath) { $rustArguments += @('--dotnet', (Resolve-InvocationPath $RuntimeHostPath)) }
    & $RustLauncher @rustArguments
    exit $LASTEXITCODE
}
$env:DOTNET_ROOT = $dotnetRoot
$env:DOTNET_CLI_TELEMETRY_OPTOUT = "1"
$env:DOTNET_NOLOGO = "1"
& $dotnetPath @previewArguments
exit $LASTEXITCODE
