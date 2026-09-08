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
$profileSchemaPath = Join-Path $repositoryRoot "schemas\template-profile-v0.1.schema.json"
$rulesPath = Join-Path $repositoryRoot "rules\ast2ir\ir-v0.1.json"
$dependencyLockPath = Join-Path $repositoryRoot "dependencies\lock.json"
$profilePath = Join-Path $repositoryRoot "profiles\templates\auri-basic\investigation-v0.1.json"
$profileTemplatePath = Join-Path $repositoryRoot "tests\fixtures\templates\minimal.hwp"
$markerProfilePath = Join-Path $repositoryRoot "profiles\templates\auri-basic\minimal-marker-investigation-v0.1.json"
$markerProfileTemplatePath = Join-Path $repositoryRoot "tests\fixtures\templates\minimal-marker.hwp"
$acceptedPath = Join-Path $repositoryRoot "examples\ir-v0.1.json"
$twoBoxesPath = Join-Path $repositoryRoot "tests\fixtures\ir\two-boxes-v0.1.json"
$twoFiguresPath = Join-Path $repositoryRoot "tests\fixtures\ir\two-figures-v0.1.json"
$rejectedPaths = @(
    (Join-Path $repositoryRoot "examples\ir-v0.1-rejected-page-break.json"),
    (Join-Path $repositoryRoot "examples\ir-v0.1-rejected-soft-break.json")
)

foreach ($path in @(
        $schemaPath,
        $rulesSchemaPath,
        $dependencySchemaPath,
        $profileSchemaPath,
        $rulesPath,
        $dependencyLockPath,
        $profilePath,
        $profileTemplatePath,
        $markerProfilePath,
        $markerProfileTemplatePath,
        $acceptedPath,
        $twoBoxesPath,
        $twoFiguresPath
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

$profileJson = Get-Content -Raw -LiteralPath $profilePath
$profileValid = $profileJson | Test-Json -SchemaFile $profileSchemaPath
if (-not $profileValid) {
    throw "The AURI investigation template profile failed schema validation."
}
$profileDocument = $profileJson | ConvertFrom-Json
$profileWithUnknownMember = $profileJson | ConvertFrom-Json -Depth 100
$profileWithUnknownMember | Add-Member -NotePropertyName "unknown" -NotePropertyValue $true
$profileWithUnknownJson = $profileWithUnknownMember | ConvertTo-Json -Depth 100
if ($profileWithUnknownJson | Test-Json -SchemaFile $profileSchemaPath -ErrorAction SilentlyContinue) {
    throw "The closed template-profile schema accepted an unknown root member."
}
$profileTemplate = Get-Item -LiteralPath $profileTemplatePath
$profileTemplateHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $profileTemplatePath).Hash
if ([int64]$profileDocument.template.identity.bytes -ne $profileTemplate.Length -or
    [string]$profileDocument.template.identity.sha256 -cne $profileTemplateHash) {
    throw "The AURI investigation profile no longer identifies the minimal HWP fixture."
}

$markerProfileJson = Get-Content -Raw -LiteralPath $markerProfilePath
$markerProfileValid = $markerProfileJson | Test-Json -SchemaFile $profileSchemaPath
if (-not $markerProfileValid) {
    throw "The AURI marker investigation template profile failed schema validation."
}
$markerProfileDocument = $markerProfileJson | ConvertFrom-Json
$markerProfileTemplate = Get-Item -LiteralPath $markerProfileTemplatePath
$markerProfileTemplateHash = (
    Get-FileHash -Algorithm SHA256 -LiteralPath $markerProfileTemplatePath
).Hash
if ([int64]$markerProfileDocument.template.identity.bytes -ne $markerProfileTemplate.Length -or
    [string]$markerProfileDocument.template.identity.sha256 -cne $markerProfileTemplateHash) {
    throw "The AURI marker profile no longer identifies its minimal HWP fixture."
}
$invalidMarkerProfile = $markerProfileJson | ConvertFrom-Json -Depth 100
$invalidMarkerProfile.selectors.insertion_target.paragraph = "unguarded"
if (($invalidMarkerProfile | ConvertTo-Json -Depth 100) |
    Test-Json -SchemaFile $profileSchemaPath -ErrorAction SilentlyContinue) {
    throw "The template-profile schema accepted an unguarded marker target."
}

$rejectedMarkerLineBreaks = 0
foreach ($markerLineBreak in @("`r", "`n", "`r`n")) {
    foreach ($markerText in @("{{MARK${markerLineBreak}ER}}", "{{MARKER}}$markerLineBreak")) {
        $invalidMarkerProfile = $markerProfileJson | ConvertFrom-Json -Depth 100
        $invalidMarkerProfile.selectors.insertion_target.marker = $markerText
        if (($invalidMarkerProfile | ConvertTo-Json -Depth 100) |
            Test-Json -SchemaFile $profileSchemaPath -ErrorAction SilentlyContinue) {
            throw "The template-profile schema accepted a marker containing CR/LF."
        }
        $rejectedMarkerLineBreaks++
    }
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

$twoFiguresJson = Get-Content -Raw -LiteralPath $twoFiguresPath
$twoFiguresAccepted = $twoFiguresJson | Test-Json -SchemaFile $schemaPath
if (-not $twoFiguresAccepted) {
    throw "The two-figure IR v0.1 fixture failed schema validation."
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
    TemplateProfile = $profileDocument.id
    TemplateProfiles = 2
    TemplateProfileValid = $profileValid -and $markerProfileValid
    TemplateIdentityValid = $true
    RejectedTemplateProfiles = 2 + $rejectedMarkerLineBreaks
    AcceptedIrExample = $accepted
    AcceptedTwoBoxesFixture = $twoBoxesAccepted
    AcceptedTwoFiguresFixture = $twoFiguresAccepted
    RejectedIrExamples = $rejectedCount
}
