#requires -Version 7.4
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$binary = Join-Path $root 'target/debug/md2hwp.exe'
$runtime = Join-Path $root '.local/dependencies/dotnet/10.0.400/dotnet.exe'
$worker = Join-Path $root 'tools/investigation/hancom-automation/ir-preview/bin/Debug/net10.0-windows/win-x64/publish/md2hwp-backend.exe'
$template = Join-Path $root 'tests/fixtures/templates/minimal-tagged-v1.hwp'
$ir = Join-Path $root 'tests/fixtures/ir/tagged-template-conformance-v0.1.json'
$work = Join-Path $root ('artifacts/runtime-smoke-' + [guid]::NewGuid().ToString('N'))
$output = Join-Path $work 'must-not-exist.hwp'
$missing = Join-Path $work 'missing-dotnet.exe'
$hash = (Get-FileHash -LiteralPath $template).Hash
$null = [IO.Directory]::CreateDirectory($work)
$standalone = Join-Path $work 'md2hwp-backend.exe'
try {
    # Move only a copy of the published EXE into an otherwise empty directory.
    Copy-Item -LiteralPath $worker -Destination $standalone
    $start = [Diagnostics.ProcessStartInfo]::new($standalone)
    $start.UseShellExecute = $false
    $start.WorkingDirectory = $work
    $start.ArgumentList.Add('runtime-info')
    $start.Environment['DOTNET_ROOT_X64'] = [IO.Path]::GetDirectoryName($runtime)
    $start.Environment['DOTNET_ROOT'] = [IO.Path]::GetDirectoryName($runtime)
    $start.Environment['DOTNET_ROLL_FORWARD_TO_PRERELEASE'] = '0'
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    try {
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Standalone worker failed: $stderr" }
        $info = $stdout | ConvertFrom-Json
        if ($info.Framework -ne 'Microsoft.NETCore.App' -or $info.Version -notmatch '^10\.0\.\d+$' -or
            $info.Architecture -ne 'X64') { throw "Wrong bundled runtime contract: $stdout" }
    } finally { $process.Dispose() }
    if (@(Get-ChildItem -LiteralPath $work -Force).Count -ne 1) { throw 'Worker required or created sidecar files.' }
    $config = Get-Content -LiteralPath (Join-Path ([IO.Path]::GetDirectoryName([IO.Path]::GetDirectoryName($worker))) 'md2hwp-backend.runtimeconfig.json') -Raw | ConvertFrom-Json
    if ($config.runtimeOptions.framework.version -ne '10.0.0' -or
        $config.runtimeOptions.rollForward -ne 'LatestPatch') { throw 'Published runtime contract drifted.' }
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
    # Run the relocated CLI from a different cwd: defaults belong beside its EXE.
    $localCli = Join-Path $work 'md2hwp.exe'
    Copy-Item -LiteralPath $binary -Destination $localCli
    Push-Location $root
    try {
        $text = & $localCli render-hwp --ir $ir --output $output --dotnet $missing 2>&1 | Out-String
        if ($LASTEXITCODE -eq 0 -or -not $text.Contains((Join-Path $work 'template.hwp'))) { throw "Wrong default template: $text" }
        Copy-Item -LiteralPath $template -Destination (Join-Path $work 'template.hwp')
        $text = & $localCli render-hwp --ir $ir --output $output --dotnet $missing 2>&1 | Out-String
        if ($LASTEXITCODE -eq 0 -or $text -notmatch 'https://dotnet.microsoft.com') { throw "Defaults did not resolve: $text" }
        Remove-Item -LiteralPath (Join-Path $work 'template.hwp')
        # Standalone backend defaults must likewise resolve beside its relocated EXE.
        $start.ArgumentList.Clear()
        foreach ($arg in @('render-tagged', '--ir', $ir, '--output', $output)) { $start.ArgumentList.Add($arg) }
        $start.WorkingDirectory = $root
        $process = [Diagnostics.Process]::Start($start)
        try {
            $stdout = $process.StandardOutput.ReadToEnd()
            $stderr = $process.StandardError.ReadToEnd()
            $process.WaitForExit()
            if ($process.ExitCode -eq 0 -or -not $stderr.Contains((Join-Path $work 'template.hwp'))) { throw "Wrong standalone default: $stderr" }
        } finally { $process.Dispose() }
        if (Test-Path -LiteralPath $output) { throw 'Failed prerequisites created output.' }
    } finally {
        Pop-Location
        Remove-Item -LiteralPath $localCli
    }
    if ((Get-FileHash -LiteralPath $template).Hash -cne $hash) { throw 'Template changed.' }
    'Runtime launch smoke passed: standalone EXE without JSON, installed runtime, missing runtime, no render output, preserved template.'
} finally {
    if (Test-Path -LiteralPath $standalone) { Remove-Item -LiteralPath $standalone }
    # Delete only the empty test directory; unexpected output is retained for inspection.
    if (@(Get-ChildItem -LiteralPath $work -Force).Count -eq 0) { [IO.Directory]::Delete($work) }
}
