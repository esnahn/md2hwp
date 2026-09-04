#requires -Version 7.0

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$dotnetWrapper = Join-Path $repositoryRoot "tools\development\dotnet.ps1"
$project = Join-Path $repositoryRoot "tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj"
$ir = Join-Path $repositoryRoot "examples\ir-v0.1.json"
$image = Join-Path $repositoryRoot "assets\sample-urban-context.png"
$nullableSourceIr = Join-Path `
    (Split-Path -Parent $ir) `
    (".ir-preview-null-source-" + [guid]::NewGuid().ToString("N") + ".json")

try {
    & pwsh -NoProfile -File $dotnetWrapper build $project --configuration Debug
    if ($LASTEXITCODE -ne 0) {
        throw "The C# IR preview project failed to build."
    }

    $planJson = & pwsh -NoProfile -File $dotnetWrapper run `
        --project $project `
        --no-build `
        -- `
        plan `
        --ir $ir
    if ($LASTEXITCODE -ne 0) {
        throw "The C# IR preview plan command failed."
    }
    $plan = $planJson | ConvertFrom-Json -Depth 100

    if ($plan.Summary.SourceBlocks -ne 5 -or
        $plan.Summary.TextOperations -ne 6 -or
        $plan.Summary.FigureOperations -ne 1 -or
        $plan.Summary.ListItems -ne 3) {
        throw "Unexpected C# IR preview summary."
    }
    $body = @($plan.Operations | Where-Object { $_.Label -eq "body" })
    if ($body.Count -ne 1) {
        throw "Expected one body preview operation."
    }
    $verbatim = @($plan.Operations | Where-Object { $_.Label -eq "verbatim_block" })
    if ($verbatim.Count -ne 1) {
        throw "Expected one verbatim-block preview operation."
    }
    $nfcUnicode = "Unicode: 가 가 도시🏙️"
    if (-not ([string]$body[0].Lines[0]).Contains($nfcUnicode)) {
        throw "The C# plan did not preserve the NFC Korean and mixed Unicode text from IR."
    }
    $figure = @($plan.Operations | Where-Object { $_.Kind -eq "figure" })
    if ($figure.Count -ne 1 -or
        [IO.Path]::GetFullPath([string]$figure[0].ImagePath) -cne [IO.Path]::GetFullPath($image) -or
        [double]$figure[0].ImageWidthMillimeters -ne 142 -or
        [Math]::Abs([double]$figure[0].ImageHeightMillimeters - (142 / 1.5)) -gt 0.000001) {
        throw "The C# figure preview plan is incorrect."
    }

    $nullableDocument = Get-Content -Raw -Encoding UTF8 -LiteralPath $ir |
        ConvertFrom-Json -Depth 100
    ($nullableDocument.blocks | Where-Object { $_.type -eq "figure" }).source = $null
    $nullableDocument |
        ConvertTo-Json -Depth 100 |
        Set-Content -Encoding UTF8 -LiteralPath $nullableSourceIr
    $nullablePlanJson = & pwsh -NoProfile -File $dotnetWrapper run `
        --project $project `
        --no-build `
        -- `
        plan `
        --ir $nullableSourceIr
    if ($LASTEXITCODE -ne 0) {
        throw "The C# IR preview rejected a nullable figure source."
    }
    $nullablePlan = $nullablePlanJson | ConvertFrom-Json -Depth 100
    $nullableFigure = @($nullablePlan.Operations | Where-Object { $_.Kind -eq "figure" })
    if ($nullableFigure.Count -ne 1 -or $nullableFigure[0].Lines[-1] -cne "source: ") {
        throw "The C# figure preview did not preserve the empty source placeholder."
    }

    [pscustomobject]@{
        DotNet = (& pwsh -NoProfile -File $dotnetWrapper --version)
        ProjectBuild = $true
        SourceBlocks = $plan.Summary.SourceBlocks
        TextOperations = $plan.Summary.TextOperations
        FigureOperations = $plan.Summary.FigureOperations
        ListItems = $plan.Summary.ListItems
        NfcUnicodePreserved = $true
        FigureAspectRatioPreserved = $true
        NullableFigureSourceAccepted = $true
        ComInvoked = $false
    }
}
finally {
    if (Test-Path -LiteralPath $nullableSourceIr) {
        Remove-Item -LiteralPath $nullableSourceIr -Force
    }
}
