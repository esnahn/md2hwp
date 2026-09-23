#requires -Version 7.4
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$binary = Join-Path $root 'target/debug/md2hwp.exe'
$runtime = Join-Path $root '.local/dependencies/dotnet/10.0.400/dotnet.exe'
$worker = Join-Path $root 'tools/investigation/hancom-automation/ir-preview/bin/Debug/net10.0-windows/Md2Hwp.HancomIrPreview.dll'
$template = Join-Path $root 'tests/fixtures/templates/minimal-tagged-v1.hwp'
$ir = Join-Path $root 'tests/fixtures/ir/tagged-template-conformance-v0.1.json'
$work = Join-Path $root ('artifacts/runtime-smoke-' + [guid]::NewGuid().ToString('N'))
$output = Join-Path $work 'must-not-exist.hwp'
$missing = Join-Path $work 'missing-dotnet.exe'
$hash = (Get-FileHash -LiteralPath $template).Hash
$null = [IO.Directory]::CreateDirectory($work)
try {
    $text = & $binary check-runtime --dotnet $runtime 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0 -or $text -notmatch '\.NET 10 x64') { throw $text }
    foreach ($arguments in @(
        @('check-runtime', '--dotnet', $missing),
        @('render-hwp', '--worker', $worker, '--ir', $ir, '--template', $template,
          '--output', $output, '--dotnet', $missing)
    )) {
        $text = & $binary @arguments 2>&1 | Out-String
        if ($LASTEXITCODE -eq 0 -or $text -notmatch 'https://dotnet.microsoft.com/ko-kr/download/dotnet/10.0' -or
            $text -notmatch 'x64') { throw "Missing installation guidance: $text" }
        if (Test-Path -LiteralPath $output) { throw 'Missing runtime published an output.' }
    }
    if ((Get-FileHash -LiteralPath $template).Hash -cne $hash) { throw 'Template changed.' }
    'Runtime launch smoke passed: installed runtime, missing runtime, no render output, preserved template.'
} finally {
    # Delete only the empty test directory; unexpected output is retained for inspection.
    if (@(Get-ChildItem -LiteralPath $work -Force).Count -eq 0) { [IO.Directory]::Delete($work) }
}
