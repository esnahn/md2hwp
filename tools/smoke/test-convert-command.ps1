#requires -Version 7.4
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$work = Join-Path $root ('artifacts/convert-command-smoke-' + [guid]::NewGuid().ToString('N'))
$wrapper = Join-Path $root 'apps/md2hwp/convert.ps1'
$template = Join-Path $root 'tests/fixtures/templates/minimal-tagged-v1.hwp'
$originalHash = (Get-FileHash -LiteralPath $template).Hash
$null = [IO.Directory]::CreateDirectory($work)
Push-Location $root
try {
    & cargo build --quiet -p md2hwp
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed.' }
    $invalid = Join-Path $work 'unsupported.md'
    [IO.File]::WriteAllText($invalid, '> Unsupported block quote')
    $existing = Join-Path $work 'existing.hwp'
    [IO.File]::WriteAllText($existing, 'must remain unchanged')
    $cases = @(
        @{Input=$invalid; Template=$template; Output=(Join-Path $work 'unsupported.hwp'); Error='BlockQuote'},
        @{Input=$invalid; Template=(Join-Path $work 'missing.hwp'); Output=(Join-Path $work 'missing.hwp'); Error='Missing input file'},
        @{Input=$invalid; Template=$template; Output=$existing; Error='Output already exists'},
        @{Input=$invalid; Template=$template; Output=$template; Error='Output must differ'},
        @{Input=$invalid; Template=$template; Output=(Join-Path $work 'wrong.hwpx'); Error='HWP input and output'}
    )
    # Resolve user paths before switching to the repository for child processes.
    Push-Location $work
    try {
        foreach ($case in $cases) {
            $diagnostic = & (Join-Path $PSHOME 'pwsh.exe') -NoProfile -File $wrapper `
                -InputPath ([IO.Path]::GetFileName($case.Input)) -Template $case.Template `
                -Output $case.Output -SkipBuild 2>&1 | Out-String
            if ($LASTEXITCODE -eq 0 -or $diagnostic -notmatch $case.Error) {
                throw "Expected failure $($case.Error), got: $diagnostic"
            }
            if ($case.Output -notin @($existing, $template) -and (Test-Path -LiteralPath $case.Output)) {
                throw 'A rejected conversion published output.'
            }
        }
    } finally { Pop-Location }
    if ([IO.File]::ReadAllText($existing) -cne 'must remain unchanged' -or
        (Get-FileHash -LiteralPath $template).Hash -cne $originalHash) {
        throw 'A protected output or template changed.'
    }
    & (Join-Path $root 'target/debug/md2hwp.exe') md2ir --from commonmark `
        --input (Join-Path $root 'examples/report-workflow-v0.2.md') --output (Join-Path $work 'report.json')
    if ($LASTEXITCODE -ne 0) { throw 'Report normalization failed.' }
    $report = Get-Content -Raw (Join-Path $work 'report.json') | ConvertFrom-Json -Depth 100
    if (@($report.blocks | Where-Object type -eq 'figure').Count -ne 2 -or
        @($report.blocks | Where-Object type -eq 'verbatim_block').Count -ne 1) {
        throw 'Report figure/box normalization differed from the fixture.'
    }
    [pscustomobject]@{RejectedWithoutCom=$cases.Count; ProtectedInputs=$true; ReportNormalized=$true}
} finally {
    Pop-Location
    Get-ChildItem -LiteralPath $work -File | ForEach-Object { Remove-Item -LiteralPath $_.FullName }
    [IO.Directory]::Delete($work)
}
