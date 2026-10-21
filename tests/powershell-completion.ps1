# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Euisoon Ahn
[CmdletBinding()]
param(
    [string] $SourcePath
)

$ErrorActionPreference = 'Stop'
if (-not $SourcePath) { $SourcePath = Join-Path $PSScriptRoot '../tools/completions/md2hwp-completion.ps1' }
$SourcePath = (Resolve-Path -LiteralPath $SourcePath).ProviderPath
$testRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) ('md2hwp-completion-' + [guid]::NewGuid().ToString('N'))))
$savedPath = $env:PATH
$assertions = 0

function Get-CompletionTexts {
    param([string] $Line, [int] $Cursor = -1)
    if ($Cursor -lt 0) { $Cursor = $Line.Length }
    # Windows PowerShell 5.1 never invokes native argument completers for a
    # standalone '--'. Exercise real completion with each letter prefix there.
    if ($PSVersionTable.PSVersion.Major -lt 6 -and $Cursor -eq $Line.Length -and $Line.EndsWith(' --')) {
        $matches = foreach ($letter in [char[]]'abcdefghijklmnopqrstuvwxyz') {
            $prefixedLine = $Line + $letter
            $prefixedResult = [System.Management.Automation.CommandCompletion]::CompleteInput($prefixedLine, $prefixedLine.Length, $null)
            $prefixedResult.CompletionMatches | ForEach-Object { $_.CompletionText }
        }
        return @($matches | Select-Object -Unique)
    }
    $result = [System.Management.Automation.CommandCompletion]::CompleteInput($Line, $Cursor, $null)
    @($result.CompletionMatches | ForEach-Object { $_.CompletionText })
}

function Assert-Contains {
    param([string[]] $Actual, [string[]] $Expected, [string] $Context)
    foreach ($value in $Expected) {
        if ($Actual -cnotcontains $value) {
            throw "$Context : missing '$value'; got [$($Actual -join ', ')]"
        }
        $script:assertions++
    }
}

function Assert-Excludes {
    param([string[]] $Actual, [string[]] $Unexpected, [string] $Context)
    foreach ($value in $Unexpected) {
        if ($Actual -ccontains $value) {
            throw "$Context : unexpected '$value'; got [$($Actual -join ', ')]"
        }
        $script:assertions++
    }
}

function Assert-FileCompletion {
    param([string] $Line, [string] $Filename, [int] $Cursor = -1)
    $actual = @(Get-CompletionTexts $Line $Cursor)
    if (-not ($actual | Where-Object { $_.Contains($Filename) })) {
        throw "$Line : expected filename '$Filename'; got [$($actual -join ', ')]"
    }
    $script:assertions++
}

try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    Push-Location -LiteralPath $testRoot
    # Empty executable stubs make native command discovery deterministic. They
    # must never be executed: every check below calls only PowerShell completion.
    New-Item -ItemType File -Path 'md2hwp.exe', 'source.md', '원고 문서.md', 'template sample.hwp' | Out-Null
    New-Item -ItemType Directory -Path 'tools with spaces' | Out-Null
    New-Item -ItemType File -Path 'tools with spaces/md2hwp.exe' | Out-Null
    $env:PATH = $testRoot + [IO.Path]::PathSeparator + $savedPath
    . $SourcePath

    foreach ($command in @('md2hwp', 'md2hwp.exe', '.\md2hwp.exe', './md2hwp.exe')) {
        $actual = @(Get-CompletionTexts "$command v")
        Assert-Contains $actual @('validate', 'validate-template') "$command subcommands"
        Assert-Excludes $actual @('md2ir', 'ir2hwp') "$command prefix"
    }
    $absoluteCommand = "& '$(Join-Path $testRoot 'tools with spaces/md2hwp.exe')'"
    Assert-Contains @(Get-CompletionTexts "$absoluteCommand v") @('validate', 'validate-template') 'quoted absolute command'
    Assert-Contains @(Get-CompletionTexts "& '.\tools with spaces\md2hwp.exe' v") @('validate', 'validate-template') 'quoted relative command'

    $actual = @(Get-CompletionTexts 'md2hwp md2ir --')
    Assert-Contains $actual @('--input', '--output', '--from', '--pandoc', '--force', '--help') 'md2ir options'
    Assert-Excludes $actual @('--worker', '--dotnet', '--template', '--legacy-com') 'md2ir options'
    $actual = @(Get-CompletionTexts 'md2hwp source.md --')
    Assert-Contains $actual @('--output', '--template', '--worker', '--dotnet', '--verbose') 'shorthand options'
    Assert-Excludes $actual @('--from', '--force', '--legacy-com', '--help') 'shorthand options'
    $actual = @(Get-CompletionTexts 'md2hwp --version --')
    Assert-Contains $actual @('--worker', '--dotnet') 'version options'
    Assert-Excludes $actual @('--output', '--template', '--from', '--verbose', '--version', '--help') 'version options'

    Assert-Contains @(Get-CompletionTexts 'md2hwp md2ir --from ') @('commonmark', 'pandoc-json') 'format values'
    $actual = @(Get-CompletionTexts 'md2hwp md2ir --from pan')
    Assert-Contains $actual @('pandoc-json') 'format prefix'
    Assert-Excludes $actual @('commonmark') 'format prefix'
    $actual = @(Get-CompletionTexts 'md2hwp md2ir --input source.md --force --')
    Assert-Excludes $actual @('--input', '--force') 'used singleton options'
    Assert-Contains $actual @('--output', '--from') 'remaining options'
    Assert-Excludes @(Get-CompletionTexts 'md2hwp md2ir source.md destination.json --') @('--input', '--output') 'positional input and output'
    Assert-Excludes @(Get-CompletionTexts 'md2hwp md2ir --input source.md destination.json --') @('--input', '--output') 'explicit input and positional output'
    Assert-Excludes @(Get-CompletionTexts 'md2hwp validate source.md --') @('--input') 'positional validation input'
    Assert-Excludes @(Get-CompletionTexts 'md2hwp validate-template "template sample.hwp" --') @('--template') 'positional template'
    Assert-Excludes @(Get-CompletionTexts 'md2hwp init-template "template sample.hwp" --') @('--output') 'positional new template'
    $actual = @(Get-CompletionTexts 'md2hwp validate --require-meta title --')
    Assert-Contains $actual @('--require-meta', '--input', '--pandoc') 'repeatable metadata option'
    $actual = @(Get-CompletionTexts 'md2hwp source.md --template "template sample.hwp" --')
    Assert-Excludes $actual @('--template') 'quoted option argument'
    Assert-Contains $actual @('--output') 'quoted option argument'

    # Both TabExpansion2 and CompleteInput exercise the registered completer.
    $tabLine = 'md2hwp validate --req'
    $tab = TabExpansion2 -inputScript $tabLine -cursorColumn $tabLine.Length
    Assert-Contains @($tab.CompletionMatches | ForEach-Object { $_.CompletionText }) @('--require-meta') 'TabExpansion2'
    $line = 'md2hwp md2ir --from pan --output later.json'
    $cursor = $line.IndexOf('pan') + 3
    Assert-Contains @(Get-CompletionTexts $line $cursor) @('pandoc-json') 'cursor before later arguments'
    $line = 'md2hwp "원고 문서.md" --tem --verbose'
    $cursor = $line.IndexOf('--tem') + '--tem'.Length
    Assert-Contains @(Get-CompletionTexts $line $cursor) @('--template') 'Unicode and middle cursor'
    Assert-Contains @(Get-CompletionTexts 'Write-Output x | md2hwp md2ir --fr') @('--from') 'command after pipe'
    Assert-Excludes @(Get-CompletionTexts 'md2hwp unknown-command --') @('--worker', '--from', '--template', '--input') 'unknown subcommand'

    Assert-FileCompletion 'md2hwp md2ir --input ./sou' 'source.md'
    Assert-FileCompletion 'md2hwp ./sou' 'source.md'
    Assert-FileCompletion 'md2hwp source.md --template "./template s' 'template sample.hwp'
    Assert-FileCompletion 'md2hwp "./원고' '원고 문서.md'

    if ($PSVersionTable.PSVersion.Major -lt 6) {
        Write-Output 'Windows PowerShell 5.1: standalone -- bypasses native completers; option checks use letter prefixes.'
    }
    Write-Output "PowerShell completion: $assertions assertions passed ($($PSVersionTable.PSVersion))."
}
finally {
    $env:PATH = $savedPath
    if ((Get-Location).ProviderPath -eq $testRoot) { Pop-Location }
    # Resolve and check the exact dedicated temporary directory before removal.
    if (Test-Path -LiteralPath $testRoot) {
        $resolvedRoot = (Resolve-Path -LiteralPath $testRoot).ProviderPath
        $tempParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/')
        if ([IO.Path]::GetDirectoryName($resolvedRoot) -ne $tempParent -or
            [IO.Path]::GetFileName($resolvedRoot) -notmatch '^md2hwp-completion-[0-9a-f]{32}$') {
            throw "Refusing to remove unexpected temporary path: $resolvedRoot"
        }
        Remove-Item -LiteralPath $resolvedRoot -Recurse -Force
    }
}
