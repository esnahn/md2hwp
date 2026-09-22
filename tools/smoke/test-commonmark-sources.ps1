#requires -Version 7.4
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$work = Join-Path $root ('artifacts/source-smoke-' + [guid]::NewGuid().ToString('N'))
$schema = Join-Path $root 'schemas/ir-v0.2.schema.json'
$utf8 = [Text.UTF8Encoding]::new($false)
$null = New-Item -ItemType Directory -Path $work
function Canonical($value) { $value | ConvertTo-Json -Depth 100 -Compress }
try {
    $actualPath = Join-Path $work 'source.ir.json'
    & cargo run --quiet -p md2hwp -- md2ir --from commonmark `
        --input (Join-Path $root 'examples/commonmark-sources-v0.2.md') --output $actualPath
    if ($LASTEXITCODE -ne 0) { throw 'CommonMark source conversion failed.' }
    $actualText = Get-Content -Raw $actualPath
    if (-not ($actualText | Test-Json -SchemaFile $schema)) { throw 'Invalid IR 0.2.' }
    $actual = $actualText | ConvertFrom-Json -Depth 100
    $expected = Get-Content -Raw (Join-Path $root 'examples/commonmark-sources-v0.2.expected.ir.json') | ConvertFrom-Json -Depth 100
    foreach ($block in $expected.blocks) {
        if ($block.type -eq 'figure') { $block.image.path = '../../assets/sample-urban-context.png' }
    }
    if ((Canonical $actual) -cne (Canonical $expected)) { throw 'CommonMark sources differ from golden IR.' }
    foreach ($block in $actual.blocks) {
        if ($block.type -eq 'figure' -and -not (Test-Path -LiteralPath (Join-Path $work $block.image.path))) {
            throw 'Rebased image does not resolve from IR output.'
        }
    }
    # The captured AST retains paths relative to its original Markdown. Rebase
    # them to this copied JSON input before testing the direct-input CLI.
    $ast = Get-Content -Raw (Join-Path $root 'tests/fixtures/pandoc-json/commonmark-sources-v0.2.json') | ConvertFrom-Json -Depth 100
    foreach ($block in $ast.blocks) {
        if ($block.t -eq 'Para' -and $block.c.Count -eq 1 -and $block.c[0].t -eq 'Image') {
            $block.c[0].c[2][0] = '../../assets/sample-urban-context.png'
        }
    }
    $jsonInput = Join-Path $work 'replay.json'
    [IO.File]::WriteAllText($jsonInput, (Canonical $ast), $utf8)
    $replayPath = Join-Path $work 'replay.ir.json'
    & cargo run --quiet -p md2hwp -- md2ir --from pandoc-json --input $jsonInput --output $replayPath
    if ($LASTEXITCODE -ne 0) { throw 'Direct JSON source replay failed.' }
    if ((Canonical (Get-Content -Raw $replayPath | ConvertFrom-Json -Depth 100)) -cne (Canonical $actual)) {
        throw 'Direct JSON replay differs from CommonMark.'
    }
    $cases = @(
        '앞 ![그림](../../assets/sample-urban-context.png)',
        "![그림](../../assets/sample-urban-context.png)`n`n출처:",
        '![그림](https://example.com/image.png)',
        "![그림](../../assets/sample-urban-context.png)`n`n출처: ``unsupported``"
    )
    for ($i = 0; $i -lt $cases.Count; $i++) {
        $inputPath = Join-Path $work "rejected-$i.md"
        $outputPath = Join-Path $work "rejected-$i.json"
        [IO.File]::WriteAllText($inputPath, $cases[$i], $utf8)
        $oldPreference = $PSNativeCommandUseErrorActionPreference
        $PSNativeCommandUseErrorActionPreference = $false
        try {
            $diagnostic = & cargo run --quiet -p md2hwp -- md2ir --from commonmark --input $inputPath --output $outputPath 2>&1 | Out-String
            $result = $LASTEXITCODE
        } finally { $PSNativeCommandUseErrorActionPreference = $oldPreference }
        if ($result -eq 0 -or (Test-Path -LiteralPath $outputPath) -or $diagnostic -notmatch '/blocks/') {
            throw "Rejected case $i left output or lacked a structural diagnostic."
        }
    }
    [pscustomobject]@{CommonMarkSources=$true;DirectJsonEquivalent=$true;RebasedImages=$true;RejectedWithoutOutput=$cases.Count;ComInvoked=$false}
} finally {
    # This directory was created above under artifacts; remove only its direct
    # test files, then its empty directory. Never recursively delete a computed path.
    Get-ChildItem -LiteralPath $work -File | ForEach-Object { Remove-Item -LiteralPath $_.FullName }
    [IO.Directory]::Delete($work)
}
