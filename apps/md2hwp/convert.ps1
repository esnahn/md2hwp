#requires -Version 7.4
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$InputPath,
    [Parameter(Mandatory)][string]$Template,
    [Parameter(Mandatory)][string]$Output,
    [ValidateSet('commonmark', 'pandoc-json')][string]$From = 'commonmark',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))

function FullPath([string]$Path) {
    $PSCmdlet.GetUnresolvedProviderPathFromPSPath($Path)
}
function Invoke-Checked([string]$Executable, [string[]]$Arguments) {
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Conversion step failed (exit $LASTEXITCODE): $Executable" }
}

if (-not $IsWindows) { throw 'The current HWP investigation requires Windows.' }
$source = FullPath $InputPath
$templatePath = FullPath $Template
$outputPath = FullPath $Output
foreach ($path in @($source, $templatePath)) {
    if (-not [IO.File]::Exists($path)) { throw "Missing input file: $path" }
}
if ([IO.Path]::GetExtension($templatePath) -ine '.hwp' -or
    [IO.Path]::GetExtension($outputPath) -ine '.hwp') {
    throw 'The current tagged-template investigation requires HWP input and output.'
}
if ($outputPath -ieq $source -or $outputPath -ieq $templatePath) {
    throw 'Output must differ from the manuscript and template.'
}
if (Test-Path -LiteralPath $outputPath) { throw "Output already exists: $outputPath" }
if (-not [IO.Directory]::Exists([IO.Path]::GetDirectoryName($outputPath))) {
    throw 'The output directory must already exist.'
}

$pwsh = Join-Path $PSHOME 'pwsh.exe'
$desktop = 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
$worker = Join-Path $root 'tools/investigation/hancom-automation/ir-preview/run-ir-preview.ps1'
$project = Join-Path $root 'tools/investigation/hancom-automation/ir-preview/Md2Hwp.HancomIrPreview.csproj'
$dotnet = Join-Path $root 'tools/development/dotnet.ps1'
$binary = Join-Path $root "target/$($Configuration.ToLowerInvariant())/md2hwp.exe"
$work = Join-Path $root ('artifacts/convert-' + [guid]::NewGuid().ToString('N'))
$ir = Join-Path $work 'document.ir.json'

Push-Location $root
try {
    if (-not $SkipBuild) {
        $cargoArgs = @('build', '--quiet', '-p', 'md2hwp')
        if ($Configuration -eq 'Release') { $cargoArgs += '--release' }
        Invoke-Checked 'cargo' $cargoArgs
    }
    if (-not [IO.File]::Exists($binary)) { throw "Missing application build: $binary" }
    $null = [IO.Directory]::CreateDirectory($work)
    # Normalization and validation finish before the Hancom process starts.
    Invoke-Checked $binary @('md2ir', '--from', $From, '--input', $source, '--output', $ir)
    if (-not $SkipBuild) {
        # Keep .NET build environment changes out of the COM worker's profile.
        Invoke-Checked $pwsh @('-NoProfile', '-File', $dotnet, 'build', $project,
            '--configuration', $Configuration, '--nologo')
    }
    # This is the existing investigation CLI, not a new production protocol.
    # The worker validates the interactive session/module, hides the window,
    # edits a copy, and publishes only after save/reopen verification.
    Invoke-Checked $desktop @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-Sta',
        '-File', $worker, '-Mode', 'render-tagged', '-Ir', $ir,
        '-Template', $templatePath, '-Output', $outputPath, '-Configuration', $Configuration)
} finally {
    if ([IO.File]::Exists($ir)) { Remove-Item -LiteralPath $ir }
    if ([IO.Directory]::Exists($work)) { [IO.Directory]::Delete($work) }
    Pop-Location
}
