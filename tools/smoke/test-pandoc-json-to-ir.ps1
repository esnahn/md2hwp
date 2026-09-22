#requires -Version 7.4

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$inputPath = Join-Path $repositoryRoot "tests\fixtures\pandoc-json\commonmark-v0.1.json"
$expectedPath = Join-Path $repositoryRoot "examples\commonmark-v0.1.expected.ir.json"
$schemaPath = Join-Path $repositoryRoot "schemas\ir-v0.2.schema.json"

foreach ($path in @($inputPath, $expectedPath, $schemaPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing Pandoc JSON smoke-test prerequisite: $path"
    }
}

$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) ("md2hwp-pandoc-json-" + [guid]::NewGuid().ToString("N"))
$actualPath = Join-Path $temporaryDirectory "actual.ir.json"
$rejectedInputPath = Join-Path $temporaryDirectory "unsupported-api.json"
$rejectedOutputPath = Join-Path $temporaryDirectory "rejected.ir.json"

try {
    $null = New-Item -ItemType Directory -Path $temporaryDirectory
    & cargo run --quiet -p md2hwp -- md2ir `
        --from pandoc-json `
        --input $inputPath `
        --output $actualPath
    if ($LASTEXITCODE -ne 0) {
        throw "The Pandoc-JSON-to-IR command failed with exit code $LASTEXITCODE."
    }

    $actualJson = Get-Content -Raw -Encoding UTF8 -LiteralPath $actualPath
    if (-not ($actualJson | Test-Json -SchemaFile $schemaPath)) {
        throw "Generated direct-input IR failed the IR 0.2 schema."
    }
    $actualCanonical = $actualJson | ConvertFrom-Json -Depth 100 | ConvertTo-Json -Depth 100 -Compress
    $expected = Get-Content -Raw -Encoding UTF8 -LiteralPath $expectedPath | ConvertFrom-Json -Depth 100
    $expected.ir_version = '0.2'
    $expectedCanonical = $expected | ConvertTo-Json -Depth 100 -Compress
    if ($actualCanonical -cne $expectedCanonical) {
        throw "Direct Pandoc JSON output differs from the expected fixture."
    }

    $unsupportedJson = (Get-Content -Raw -Encoding UTF8 -LiteralPath $inputPath) -replace `
        '"pandoc-api-version":\[1,23,1,2\]',
        '"pandoc-api-version":[9,9,9]'
    Set-Content -Encoding UTF8 -LiteralPath $rejectedInputPath -Value $unsupportedJson

    $oldNativePreference = $PSNativeCommandUseErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false
    try {
        $rejection = & cargo run --quiet -p md2hwp -- md2ir `
            --from pandoc-json `
            --input $rejectedInputPath `
            --output $rejectedOutputPath 2>&1 | Out-String
        $rejectionExitCode = $LASTEXITCODE
    }
    finally {
        $PSNativeCommandUseErrorActionPreference = $oldNativePreference
    }
    if ($rejectionExitCode -eq 0 -or $rejection -notmatch "unsupported_pandoc_api_version") {
        throw "Unsupported Pandoc API version did not produce the expected diagnostic."
    }
    if (Test-Path -LiteralPath $rejectedOutputPath) {
        throw "Rejected direct Pandoc JSON conversion left partial IR output."
    }

    [pscustomobject]@{
        Input = $inputPath
        Expected = $expectedPath
        PandocExecutableRequired = $false
        GeneratedIrValid = $true
        FixtureEquivalent = $true
        UnsupportedApiRejected = $true
        RejectionLeftNoOutput = $true
    }
}
finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}
