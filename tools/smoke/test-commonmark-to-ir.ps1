#requires -Version 7.4

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$inputPath = Join-Path $repositoryRoot "examples\commonmark-v0.1.md"
$expectedPath = Join-Path $repositoryRoot "examples\commonmark-v0.1.expected.ir.json"
$pandocPath = Join-Path $repositoryRoot ".local\dependencies\pandoc\3.10.1\pandoc.exe"
$schemaPath = Join-Path $repositoryRoot "schemas\ir-v0.1.schema.json"

foreach ($path in @($inputPath, $expectedPath, $pandocPath, $schemaPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing CommonMark smoke-test prerequisite: $path"
    }
}

$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) ("md2hwp-commonmark-" + [guid]::NewGuid().ToString("N"))
$actualPath = Join-Path $temporaryDirectory "actual.ir.json"
$rejectedInputPath = Join-Path $temporaryDirectory "rejected.md"
$rejectedOutputPath = Join-Path $temporaryDirectory "rejected.ir.json"

try {
    $null = New-Item -ItemType Directory -Path $temporaryDirectory
    & cargo run --quiet -p md2hwp -- md2ir `
        --from commonmark `
        --input $inputPath `
        --output $actualPath `
        --pandoc $pandocPath
    if ($LASTEXITCODE -ne 0) {
        throw "The CommonMark-to-IR command failed with exit code $LASTEXITCODE."
    }

    $actualJson = Get-Content -Raw -Encoding UTF8 -LiteralPath $actualPath
    if (-not ($actualJson | Test-Json -SchemaFile $schemaPath)) {
        throw "Generated CommonMark IR failed the IR v0.1 schema."
    }
    $actualCanonical = $actualJson | ConvertFrom-Json -Depth 100 | ConvertTo-Json -Depth 100 -Compress
    $expectedCanonical = Get-Content -Raw -Encoding UTF8 -LiteralPath $expectedPath |
        ConvertFrom-Json -Depth 100 |
        ConvertTo-Json -Depth 100 -Compress
    if ($actualCanonical -cne $expectedCanonical) {
        throw "Generated CommonMark IR differs from the expected fixture."
    }

    $decomposed = [string][char]0x1100 + [char]0x1161
    $sourceText = Get-Content -Raw -Encoding UTF8 -LiteralPath $inputPath
    if (-not $sourceText.Contains($decomposed)) {
        throw "The CommonMark fixture no longer contains the decomposed Unicode probe."
    }
    if ($actualJson.Contains($decomposed)) {
        throw "Pinned CommonMark parser behavior changed: decomposed probe unexpectedly remained decomposed."
    }

    Set-Content -Encoding UTF8 -LiteralPath $rejectedInputPath -Value "> unsupported block quote"
    $oldNativePreference = $PSNativeCommandUseErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false
    try {
        $rejection = & cargo run --quiet -p md2hwp -- md2ir `
            --from commonmark `
            --input $rejectedInputPath `
            --output $rejectedOutputPath `
            --pandoc $pandocPath 2>&1 | Out-String
        $rejectionExitCode = $LASTEXITCODE
    }
    finally {
        $PSNativeCommandUseErrorActionPreference = $oldNativePreference
    }
    if ($rejectionExitCode -eq 0 -or $rejection -notmatch "unsupported_pandoc_node") {
        throw "Unsupported CommonMark syntax did not produce the expected diagnostic."
    }
    if (Test-Path -LiteralPath $rejectedOutputPath) {
        throw "Rejected CommonMark conversion left partial IR output."
    }

    [pscustomobject]@{
        Pandoc = (& $pandocPath --version | Select-Object -First 1)
        Input = $inputPath
        Expected = $expectedPath
        GeneratedIrValid = $true
        FixtureEquivalent = $true
        CommonMarkCompositionObserved = $true
        UnsupportedSyntaxRejected = $true
        RejectionLeftNoOutput = $true
    }
}
finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}
