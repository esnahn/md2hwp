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
    if ([string]$body[0].ParagraphStyle -cne "body") {
        throw "The body operation lost its explicit paragraph-style binding."
    }
    $formattedBody = @($body[0].FormattedLines)
    if ($formattedBody.Count -ne @($body[0].Lines).Count) {
        throw "The body operation lost its formatted line structure."
    }
    for ($lineIndex = 0; $lineIndex -lt $formattedBody.Count; $lineIndex++) {
        $runText = -join @($formattedBody[$lineIndex] | ForEach-Object { [string]$_.Text })
        if ($runText -cne [string]$body[0].Lines[$lineIndex]) {
            throw "Formatted runs do not reconstruct body line $lineIndex."
        }
    }
    $bodyRuns = @($formattedBody | ForEach-Object { $_ })
    $strongOnly = @($bodyRuns | Where-Object {
        ([string]$_.Text).Trim() -ceq "굵게와" -and $_.Strong -and -not $_.Emphasis
    })
    $strongEmphasis = @($bodyRuns | Where-Object {
        ([string]$_.Text).Trim() -ceq "굵고 기울임" -and $_.Strong -and $_.Emphasis
    })
    $emphasisOnly = @($bodyRuns | Where-Object {
        ([string]$_.Text).Trim() -ceq "기울임" -and -not $_.Strong -and $_.Emphasis
    })
    $markedLinkLabel = @($bodyRuns | Where-Object {
        [string]$_.Text -ceq "AURI" -and $_.Strong -and -not $_.Emphasis
    })
    if ($strongOnly.Count -ne 1 -or
        $strongEmphasis.Count -ne 1 -or
        $emphasisOnly.Count -ne 1 -or
        $markedLinkLabel.Count -ne 1) {
        throw "Nested strong/emphasis or recursively formatted link-label runs were not preserved."
    }
    $listParagraphs = @($plan.Operations | Where-Object { ([string]$_.Label).StartsWith("list.") })
    if ($listParagraphs.Count -ne 3 -or
        @($listParagraphs | Where-Object { [string]$_.ParagraphStyle -cne "body" }).Count -ne 0) {
        throw "List context must not be inferred from a style-name prefix; every list paragraph must bind body explicitly."
    }

    $verbatim = @($plan.Operations | Where-Object { $_.Label -eq "verbatim_block" })
    if ($verbatim.Count -ne 1 -or $verbatim[0].ParagraphStyle -cne "block.box") {
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

    $figureLines = @($figure[0].FormattedLines)
    $captionRuns = @($figureLines[1])
    if ($figureLines.Count -ne 3 -or
        $captionRuns.Count -lt 1 -or
        [string]$captionRuns[0].Text -cne "도시" -or
        -not $captionRuns[0].Strong -or
        $captionRuns[0].Emphasis) {
        throw "The figure caption lost its strong character-mark run."
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
    if ($nullableFigure.Count -ne 1 -or $nullableFigure[0].Lines[-1] -cne "") {
        throw "The C# figure preview did not preserve the empty source placeholder."
    }

    [pscustomobject]@{
        DotNet = (& pwsh -NoProfile -File $dotnetWrapper --version)
        ProjectBuild = $true
        SourceBlocks = $plan.Summary.SourceBlocks
        TextOperations = $plan.Summary.TextOperations
        FigureOperations = $plan.Summary.FigureOperations
        ListItems = $plan.Summary.ListItems
        ExplicitListBodyStyles = $true
        StrongEmphasisRunsPreserved = $true
        FigureCaptionRunsPreserved = $true
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
