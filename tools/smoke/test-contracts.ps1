#requires -Version 7.0

# Validates product data contracts, the dependency lock, and IR examples.
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$schemaPath = Join-Path $repositoryRoot "schemas\ir-v0.1.schema.json"
$rulesSchemaPath = Join-Path $repositoryRoot "schemas\ast2ir-rules-v0.1.schema.json"
$dependencySchemaPath = Join-Path $repositoryRoot "schemas\dependencies-lock-v0.1.schema.json"
$rulesPath = Join-Path $repositoryRoot "rules\ast2ir\ir-v0.1.json"
$dependencyLockPath = Join-Path $repositoryRoot "dependencies\lock.json"
$acceptedPath = Join-Path $repositoryRoot "examples\ir-v0.1.json"
$twoBoxesPath = Join-Path $repositoryRoot "tests\fixtures\ir\two-boxes-v0.1.json"
$rejectedPaths = @(
    (Join-Path $repositoryRoot "examples\ir-v0.1-rejected-page-break.json"),
    (Join-Path $repositoryRoot "examples\ir-v0.1-rejected-soft-break.json")
)

foreach ($path in @(
        $schemaPath,
        $rulesSchemaPath,
        $dependencySchemaPath,
        $rulesPath,
        $dependencyLockPath,
        $acceptedPath,
        $twoBoxesPath
    ) + $rejectedPaths) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing smoke-test input: $path"
    }
}

$rulesJson = Get-Content -Raw -LiteralPath $rulesPath
$rulesValid = $rulesJson | Test-Json -SchemaFile $rulesSchemaPath
if (-not $rulesValid) {
    throw "The AST-to-IR v0.1 ruleset failed schema validation."
}

$dependencyDataPaths = @(
    Get-ChildItem -LiteralPath (Split-Path $dependencyLockPath -Parent) `
        -Recurse -File -Filter "*.json"
)
if ($dependencyDataPaths.Count -ne 1 -or
    $dependencyDataPaths[0].FullName -ne [IO.Path]::GetFullPath($dependencyLockPath)) {
    throw "dependencies/ must contain exactly one JSON data file: lock.json."
}

$dependencyJson = Get-Content -Raw -LiteralPath $dependencyLockPath
$dependencyValid = $dependencyJson | Test-Json -SchemaFile $dependencySchemaPath
if (-not $dependencyValid) {
    throw "The external dependencies lock failed schema validation."
}
$dependencyDocument = $dependencyJson | ConvertFrom-Json
$dependencyNames = @($dependencyDocument.dependencies | ForEach-Object { $_.name })
$uniqueDependencyNames = @($dependencyNames | Sort-Object -Unique)
if ($dependencyNames.Count -ne $uniqueDependencyNames.Count) {
    throw "The external dependencies lock contains duplicate dependency names."
}

$acceptedJson = Get-Content -Raw -LiteralPath $acceptedPath
$accepted = $acceptedJson | Test-Json -SchemaFile $schemaPath
if (-not $accepted) {
    throw "The accepted IR v0.1 example failed schema validation."
}

$twoBoxesJson = Get-Content -Raw -LiteralPath $twoBoxesPath
$twoBoxesAccepted = $twoBoxesJson | Test-Json -SchemaFile $schemaPath
if (-not $twoBoxesAccepted) {
    throw "The two-box IR v0.1 fixture failed schema validation."
}

$rejectedCount = 0
foreach ($rejectedPath in $rejectedPaths) {
    $rejectedJson = Get-Content -Raw -LiteralPath $rejectedPath
    $rejected = $rejectedJson | Test-Json -SchemaFile $schemaPath -ErrorAction SilentlyContinue
    if ($rejected) {
        throw "A rejected IR example was unexpectedly accepted: $rejectedPath"
    }
    $rejectedCount++
}

[pscustomobject]@{
    IrSchema = $schemaPath
    Ast2IrRules = $rulesValid
    ExternalDependencies = $dependencyDocument.dependencies.Count
    AcceptedIrExample = $accepted
    AcceptedTwoBoxesFixture = $twoBoxesAccepted
    RejectedIrExamples = $rejectedCount
}
