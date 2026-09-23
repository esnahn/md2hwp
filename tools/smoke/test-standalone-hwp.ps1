#requires -Version 5.1
# Explicit live test, never part of the ordinary build/test suite.
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($PSVersionTable.PSEdition -ne 'Desktop' -or -not [Environment]::Is64BitProcess -or
    -not [Environment]::UserInteractive -or [Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0 -or
    [Threading.Thread]::CurrentThread.ApartmentState -ne 'STA') { throw 'Use the verified interactive Windows PowerShell 5.1 x64 STA context.' }
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$work = Join-Path ([IO.Path]::GetTempPath()) ('md2hwp-standalone-' + [guid]::NewGuid().ToString('N'))
$null = [IO.Directory]::CreateDirectory((Join-Path $work 'examples'))
$null = [IO.Directory]::CreateDirectory((Join-Path $work 'assets'))
Copy-Item -LiteralPath (Join-Path $repo 'target/debug/md2hwp.exe') -Destination (Join-Path $work 'md2hwp.exe')
Copy-Item -LiteralPath (Join-Path $repo 'tools/investigation/hancom-automation/ir-preview/bin/Debug/net10.0-windows/win-x64/publish/md2hwp-backend.exe') -Destination (Join-Path $work 'md2hwp-backend.exe')
Copy-Item -LiteralPath (Join-Path $repo 'tests/fixtures/templates/minimal-tagged-v1.hwp') -Destination (Join-Path $work 'template.hwp')
Copy-Item -LiteralPath (Join-Path $repo 'examples/report-workflow-v0.2.md') -Destination (Join-Path $work 'examples/report.md')
Copy-Item -LiteralPath (Join-Path $repo 'assets/sample-urban-context.png') -Destination (Join-Path $work 'assets/sample-urban-context.png')
$runtime = Join-Path $repo '.local/dependencies/dotnet/10.0.400/dotnet.exe'
Push-Location $work
try {
    & .\md2hwp.exe md2ir --from commonmark --input .\examples\report.md --output .\document.ir.json
    if ($LASTEXITCODE -ne 0) { throw 'Standalone Markdown normalization failed.' }
    & .\md2hwp.exe render-hwp --ir .\document.ir.json --output .\result.hwp --dotnet $runtime
    if ($LASTEXITCODE -ne 0) { throw 'Standalone HWP render failed.' }
    if (Test-Path -LiteralPath .\dependencies\lock.json) { throw 'Test unexpectedly included a lock file.' }
    Write-Output "Standalone proof retained at: $work"
} finally { Pop-Location }
