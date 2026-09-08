#requires -Version 5.1

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$File,

    [string]$RootIndexes = '',

    [string]$TableIndexes = '',

    [string]$StyleNames = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$requiredHost = 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
$actualHost = [IO.Path]::GetFullPath(
    [Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
)
if ($PSVersionTable.PSEdition -ne 'Desktop' -or
    $PSVersionTable.PSVersion.Major -ne 5 -or
    $PSVersionTable.PSVersion.Minor -ne 1 -or
    -not [Environment]::Is64BitProcess -or
    [Threading.Thread]::CurrentThread.ApartmentState -ne [Threading.ApartmentState]::STA -or
    -not [Environment]::UserInteractive -or
    [Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0 -or
    -not $actualHost.Equals($requiredHost, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Run in interactive Windows PowerShell 5.1 x64 STA.'
}

function Get-Sha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    try {
        $sha = [Security.Cryptography.SHA256]::Create()
        try {
            return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '')
        }
        finally {
            $sha.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Get-AttributeMap($Element) {
    $result = [ordered]@{}
    foreach ($attribute in @($Element.Attributes)) {
        $result[[string]$attribute.Name] = [string]$attribute.Value
    }
    return [pscustomobject]$result
}

function Get-ParagraphSummary($Paragraph, [int]$RootIndex, [hashtable]$Styles) {
    $styleId = -1
    [void][int]::TryParse([string]$Paragraph.GetAttribute('Style'), [ref]$styleId)
    $styleName = if ($Styles.ContainsKey($styleId)) {
        [string]$Styles[$styleId]
    }
    else {
        $null
    }
    $text = ([string]$Paragraph.InnerText -replace '\s+', ' ').Trim()
    if ($text.Length -gt 140) {
        $text = $text.Substring(0, 140) + '...'
    }
    $ancestorNames = @()
    $ancestor = $Paragraph.ParentNode
    while ($null -ne $ancestor -and $ancestor.NodeType -eq [Xml.XmlNodeType]::Element) {
        $ancestorNames += [string]$ancestor.LocalName
        $ancestor = $ancestor.ParentNode
    }
    $descendantStyleNames = @(
        $Paragraph.SelectNodes(".//*[local-name()='P']") |
            ForEach-Object {
                $id = -1
                [void][int]::TryParse([string]$_.GetAttribute('Style'), [ref]$id)
                if ($Styles.ContainsKey($id)) { [string]$Styles[$id] } else { "#$id" }
            } |
            Group-Object |
            Sort-Object Name |
            ForEach-Object { "$($_.Name):$($_.Count)" }
    )
    [pscustomobject]@{
        RootIndex = $RootIndex
        StyleId = $styleId
        StyleName = $styleName
        Ancestors = $ancestorNames
        Tables = @($Paragraph.SelectNodes(".//*[local-name()='TABLE']")).Count
        Pictures = @($Paragraph.SelectNodes(".//*[local-name()='PICTURE']")).Count
        FigureAutoNumbers = @(
            $Paragraph.SelectNodes(".//*[local-name()='AUTONUM']") |
                Where-Object { $_.GetAttribute('NumberType') -eq 'Figure' }
        ).Count
        DescendantParagraphStyles = $descendantStyleNames
        Text = $text
    }
}

$absoluteFile = [IO.Path]::GetFullPath($File)
if (-not (Test-Path -LiteralPath $absoluteFile -PathType Leaf)) {
    throw "Missing HWP: $absoluteFile"
}
if (@(Get-Process Hwp -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'Refusing to inspect while HWP is already running.'
}

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\..'))
$lock = Get-Content -Raw -Encoding UTF8 -LiteralPath (
    Join-Path $repositoryRoot 'dependencies\lock.json'
) | ConvertFrom-Json
$pin = @(
    $lock.dependencies |
        Where-Object { $_.name -eq 'hancom-automation' } |
        Select-Object -ExpandProperty pins |
        Where-Object { $_.name -eq 'file-path-checker-module-example' }
)
if ($pin.Count -ne 1) {
    throw 'Expected one file-path checker pin.'
}
$registryPath = 'HKCU:\Software\HNC\HwpAutomation\Modules'
$key = Get-Item -LiteralPath $registryPath
$modulePath = [string]$key.GetValue(
    'FilePathCheckerModuleExample',
    $null,
    [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
)
if (-not [IO.Path]::IsPathRooted($modulePath) -or
    -not (Test-Path -LiteralPath $modulePath -PathType Leaf) -or
    (Get-Sha256 $modulePath) -ne [string]$pin[0].sha256) {
    throw 'Registered security module does not match dependencies/lock.json.'
}

$hashBefore = Get-Sha256 $absoluteFile
$hwp = $null
$opened = $false
try {
    $hwp = New-Object -ComObject 'HWPFrame.HwpObject'
    if (-not [bool]$hwp.RegisterModule('FilePathCheckDLL', 'FilePathCheckerModuleExample')) {
        throw 'RegisterModule failed.'
    }
    $opened = [bool]$hwp.Open(
        $absoluteFile,
        'HWP',
        'lock:false;forceopen:true;suspendpassword:true;versionwarning:false'
    )
    if (-not $opened) {
        throw 'Open failed.'
    }

    [xml]$document = [string]$hwp.GetTextFile('HWPML2X', '')
    $styles = @{}
    foreach ($style in @($document.SelectNodes("//*[local-name()='STYLE']"))) {
        $id = -1
        if ([int]::TryParse([string]$style.GetAttribute('Id'), [ref]$id)) {
            $styles[$id] = [string]$style.GetAttribute('Name')
        }
    }
    $sections = @($document.SelectNodes("//*[local-name()='SECTION']"))
    $rootParagraphs = @()
    foreach ($section in $sections) {
        $rootParagraphs += @($section.SelectNodes("./*[local-name()='P']"))
    }
    $allParagraphs = @($document.SelectNodes("//*[local-name()='P']"))

    $styleUsage = @(
        $allParagraphs |
            ForEach-Object {
                $id = -1
                [void][int]::TryParse([string]$_.GetAttribute('Style'), [ref]$id)
                [pscustomobject]@{
                    Id = $id
                    Name = if ($styles.ContainsKey($id)) { [string]$styles[$id] } else { $null }
                }
            } |
            Group-Object Id |
            ForEach-Object {
                [pscustomobject]@{
                    Id = [int]$_.Name
                    Name = $_.Group[0].Name
                    Paragraphs = $_.Count
                }
            } |
            Sort-Object Id
    )

    $interestingRoots = @()
    for ($index = 0; $index -lt $rootParagraphs.Count; $index++) {
        $paragraph = $rootParagraphs[$index]
        if (@($paragraph.SelectNodes(
            ".//*[local-name()='TABLE' or local-name()='PICTURE' or local-name()='AUTONUM']"
        )).Count -gt 0) {
            $interestingRoots += Get-ParagraphSummary $paragraph $index $styles
        }
    }

    $fieldLike = @(
        $document.SelectNodes('//*') |
            Where-Object { $_.LocalName -match '(?i)(field|bookmark)' } |
            ForEach-Object {
                [pscustomobject]@{
                    Element = [string]$_.LocalName
                    Attributes = Get-AttributeMap $_
                    Text = (([string]$_.InnerText -replace '\s+', ' ').Trim())
                }
            }
    )

    $elementCounts = @(
        $document.SelectNodes('//*') |
            Group-Object LocalName |
            Sort-Object Name |
            ForEach-Object {
                [pscustomobject]@{ Element = $_.Name; Count = $_.Count }
            }
    )

    $requestedRootIndexes = @(
        $RootIndexes.Split(',', [StringSplitOptions]::RemoveEmptyEntries) |
            ForEach-Object { [int]$_.Trim() }
    )
    $requestedTableIndexes = @(
        $TableIndexes.Split(',', [StringSplitOptions]::RemoveEmptyEntries) |
            ForEach-Object { [int]$_.Trim() }
    )
    $requestedStyleNames = @(
        $StyleNames.Split('|', [StringSplitOptions]::RemoveEmptyEntries) |
            ForEach-Object { $_.Trim() }
    )
    $targetRoots = @(
        foreach ($rootIndex in $requestedRootIndexes) {
            if ($rootIndex -lt 0 -or $rootIndex -ge $rootParagraphs.Count) {
                throw "Root index is outside the document: $rootIndex"
            }
            $root = $rootParagraphs[$rootIndex]
            $paragraphDetails = @(
                @($root) + @($root.SelectNodes(".//*[local-name()='P']")) |
                    ForEach-Object {
                        $styleId = -1
                        [void][int]::TryParse([string]$_.GetAttribute('Style'), [ref]$styleId)
                        $paragraphText = (([string]$_.InnerText -replace '\s+', ' ').Trim())
                        if ($paragraphText.Length -gt 240) {
                            $paragraphText = $paragraphText.Substring(0, 240) + '...'
                        }
                        [pscustomobject]@{
                            Parent = [string]$_.ParentNode.LocalName
                            StyleId = $styleId
                            StyleName = if ($styles.ContainsKey($styleId)) {
                                [string]$styles[$styleId]
                            }
                            else {
                                $null
                            }
                            Tables = @($_.SelectNodes(".//*[local-name()='TABLE']")).Count
                            Pictures = @($_.SelectNodes(".//*[local-name()='PICTURE']")).Count
                            AutoNumbers = @($_.SelectNodes(".//*[local-name()='AUTONUM']")).Count
                            Text = $paragraphText
                        }
                    }
            )
            $controls = @(
                $root.SelectNodes(
                    ".//*[local-name()='TABLE' or local-name()='PICTURE' or " +
                    "local-name()='CAPTION' or local-name()='AUTONUM' or " +
                    "local-name()='POSITION']"
                ) |
                    ForEach-Object {
                        [pscustomobject]@{
                            Element = [string]$_.LocalName
                            Parent = [string]$_.ParentNode.LocalName
                            Attributes = Get-AttributeMap $_
                            Text = if ($_.LocalName -eq 'AUTONUM') {
                                [string]$_.InnerText
                            }
                            else {
                                $null
                            }
                        }
                    }
            )
            [pscustomobject]@{
                Summary = Get-ParagraphSummary $root $rootIndex $styles
                Paragraphs = $paragraphDetails
                Controls = $controls
            }
        }
    )
    $tables = @($document.SelectNodes("//*[local-name()='TABLE']"))
    $targetTables = @(
        foreach ($tableIndex in $requestedTableIndexes) {
            if ($tableIndex -lt 0 -or $tableIndex -ge $tables.Count) {
                throw "Table index is outside the document: $tableIndex"
            }
            $table = $tables[$tableIndex]
            $rows = @($table.SelectNodes(".//*[local-name()='ROW']"))
            [pscustomobject]@{
                TableIndex = $tableIndex
                Attributes = Get-AttributeMap $table
                Rows = @(
                    for ($rowIndex = 0; $rowIndex -lt $rows.Count; $rowIndex++) {
                        $cells = @($rows[$rowIndex].SelectNodes("./*[local-name()='CELL']"))
                        [pscustomobject]@{
                            RowIndex = $rowIndex
                            Cells = @(
                                for ($cellIndex = 0; $cellIndex -lt $cells.Count; $cellIndex++) {
                                    $cell = $cells[$cellIndex]
                                    [pscustomobject]@{
                                        CellIndex = $cellIndex
                                        Attributes = Get-AttributeMap $cell
                                        Text = (([string]$cell.InnerText -replace '\s+', ' ').Trim())
                                        ParagraphStyles = @(
                                            $cell.SelectNodes(".//*[local-name()='P']") |
                                                ForEach-Object {
                                                    $id = -1
                                                    [void][int]::TryParse(
                                                        [string]$_.GetAttribute('Style'),
                                                        [ref]$id
                                                    )
                                                    if ($styles.ContainsKey($id)) {
                                                        [string]$styles[$id]
                                                    }
                                                    else {
                                                        "#$id"
                                                    }
                                                } |
                                                Select-Object -Unique
                                        )
                                    }
                                }
                            )
                        }
                    }
                )
            }
        }
    )
    $charShapes = @{}
    foreach ($shape in @($document.SelectNodes("//*[local-name()='CHARSHAPE']"))) {
        $id = -1
        if ([int]::TryParse([string]$shape.GetAttribute('Id'), [ref]$id)) {
            $charShapes[$id] = $shape
        }
    }
    $paraShapes = @{}
    foreach ($shape in @($document.SelectNodes("//*[local-name()='PARASHAPE']"))) {
        $id = -1
        if ([int]::TryParse([string]$shape.GetAttribute('Id'), [ref]$id)) {
            $paraShapes[$id] = $shape
        }
    }
    $styleElements = @($document.SelectNodes("//*[local-name()='STYLE']"))
    $hangulFonts = @{}
    foreach ($fontFace in @(
        $document.SelectNodes("//*[local-name()='FONTFACE' and @Lang='Hangul']")
    )) {
        foreach ($font in @($fontFace.SelectNodes(".//*[local-name()='FONT']"))) {
            $fontId = -1
            if ([int]::TryParse([string]$font.GetAttribute('Id'), [ref]$fontId)) {
                $hangulFonts[$fontId] = [string]$font.GetAttribute('Name')
            }
        }
    }
    $targetStyles = @(
        foreach ($requestedStyleName in $requestedStyleNames) {
            $matches = @(
                $styleElements |
                    Where-Object {
                        [string]$_.GetAttribute('Name') -ceq $requestedStyleName
                    }
            )
            if ($matches.Count -ne 1) {
                throw "Expected one style named '$requestedStyleName'; found $($matches.Count)."
            }
            $style = $matches[0]
            $charShapeId = -1
            $paraShapeId = -1
            [void][int]::TryParse(
                [string]$style.GetAttribute('CharShape'),
                [ref]$charShapeId
            )
            [void][int]::TryParse(
                [string]$style.GetAttribute('ParaShape'),
                [ref]$paraShapeId
            )
            $charShape = if ($charShapes.ContainsKey($charShapeId)) {
                $charShapes[$charShapeId]
            }
            else { $null }
            $paraShape = if ($paraShapes.ContainsKey($paraShapeId)) {
                $paraShapes[$paraShapeId]
            }
            else { $null }
            $fontIdElement = if ($null -eq $charShape) {
                $null
            }
            else {
                $charShape.SelectSingleNode("./*[local-name()='FONTID']")
            }
            $ratioElement = if ($null -eq $charShape) {
                $null
            }
            else {
                $charShape.SelectSingleNode("./*[local-name()='RATIO']")
            }
            $spacingElement = if ($null -eq $charShape) {
                $null
            }
            else {
                $charShape.SelectSingleNode("./*[local-name()='CHARSPACING']")
            }
            $marginElement = if ($null -eq $paraShape) {
                $null
            }
            else {
                $paraShape.SelectSingleNode("./*[local-name()='PARAMARGIN']")
            }
            $hangulFontId = -1
            if ($null -ne $fontIdElement) {
                [void][int]::TryParse(
                    [string]$fontIdElement.GetAttribute('Hangul'),
                    [ref]$hangulFontId
                )
            }
            [pscustomobject]@{
                Name = $requestedStyleName
                Style = Get-AttributeMap $style
                Summary = [pscustomobject]@{
                    Height = if ($null -eq $charShape) {
                        $null
                    }
                    else { [string]$charShape.GetAttribute('Height') }
                    HangulFontId = $hangulFontId
                    HangulFont = if ($hangulFonts.ContainsKey($hangulFontId)) {
                        [string]$hangulFonts[$hangulFontId]
                    }
                    else { $null }
                    HangulRatio = if ($null -eq $ratioElement) {
                        $null
                    }
                    else { [string]$ratioElement.GetAttribute('Hangul') }
                    HangulSpacing = if ($null -eq $spacingElement) {
                        $null
                    }
                    else { [string]$spacingElement.GetAttribute('Hangul') }
                    Bold = ($null -ne $charShape -and
                        $null -ne $charShape.SelectSingleNode("./*[local-name()='BOLD']"))
                    Italic = ($null -ne $charShape -and
                        $null -ne $charShape.SelectSingleNode("./*[local-name()='ITALIC']"))
                    Align = if ($null -eq $paraShape) {
                        $null
                    }
                    else { [string]$paraShape.GetAttribute('Align') }
                    HeadingType = if ($null -eq $paraShape) {
                        $null
                    }
                    else { [string]$paraShape.GetAttribute('HeadingType') }
                    Level = if ($null -eq $paraShape) {
                        $null
                    }
                    else { [string]$paraShape.GetAttribute('Level') }
                    LineSpacing = if ($null -eq $marginElement) {
                        $null
                    }
                    else { [string]$marginElement.GetAttribute('LineSpacing') }
                    LineSpacingType = if ($null -eq $marginElement) {
                        $null
                    }
                    else { [string]$marginElement.GetAttribute('LineSpacingType') }
                    Left = if ($null -eq $marginElement) {
                        $null
                    }
                    else { [string]$marginElement.GetAttribute('Left') }
                    Right = if ($null -eq $marginElement) {
                        $null
                    }
                    else { [string]$marginElement.GetAttribute('Right') }
                    Indent = if ($null -eq $marginElement) {
                        $null
                    }
                    else { [string]$marginElement.GetAttribute('Indent') }
                    Prev = if ($null -eq $marginElement) {
                        $null
                    }
                    else { [string]$marginElement.GetAttribute('Prev') }
                    Next = if ($null -eq $marginElement) {
                        $null
                    }
                    else { [string]$marginElement.GetAttribute('Next') }
                }
                CharShape = if ($null -eq $charShape) {
                    $null
                }
                else {
                    [pscustomobject]@{
                        Attributes = Get-AttributeMap $charShape
                        Children = @(
                            $charShape.ChildNodes |
                                Where-Object { $_.NodeType -eq [Xml.XmlNodeType]::Element } |
                                ForEach-Object {
                                    [pscustomobject]@{
                                        Element = [string]$_.LocalName
                                        Attributes = Get-AttributeMap $_
                                    }
                                }
                        )
                    }
                }
                ParaShape = if ($null -eq $paraShape) {
                    $null
                }
                else {
                    [pscustomobject]@{
                        Attributes = Get-AttributeMap $paraShape
                        Children = @(
                            $paraShape.ChildNodes |
                                Where-Object { $_.NodeType -eq [Xml.XmlNodeType]::Element } |
                                ForEach-Object {
                                    [pscustomobject]@{
                                        Element = [string]$_.LocalName
                                        Attributes = Get-AttributeMap $_
                                    }
                                }
                        )
                    }
                }
            }
        }
    )

    [object[]]$reportedStyles = @()
    [object[]]$reportedStyleUsage = @()
    [object[]]$reportedInterestingRoots = @()
    [object[]]$reportedElementCounts = @()
    if ($requestedRootIndexes.Count -eq 0 -and
        $requestedTableIndexes.Count -eq 0 -and
        $requestedStyleNames.Count -eq 0) {
        $reportedStyles = @(
            $styleElements |
                Sort-Object { [int]$_.GetAttribute('Id') } |
                ForEach-Object {
                    [pscustomobject]@{
                        Id = [int]$_.GetAttribute('Id')
                        Name = [string]$_.GetAttribute('Name')
                        CharShape = [int]$_.GetAttribute('CharShape')
                        ParaShape = [int]$_.GetAttribute('ParaShape')
                    }
                }
        )
        $reportedStyleUsage = $styleUsage
        $reportedInterestingRoots = $interestingRoots
        $reportedElementCounts = $elementCounts
    }

    [pscustomobject]@{
        File = $absoluteFile
        Bytes = (Get-Item -LiteralPath $absoluteFile).Length
        Sha256 = $hashBefore
        Sections = $sections.Count
        RootParagraphs = $rootParagraphs.Count
        AllParagraphs = $allParagraphs.Count
        Tables = @($document.SelectNodes("//*[local-name()='TABLE']")).Count
        Pictures = @($document.SelectNodes("//*[local-name()='PICTURE']")).Count
        FigureAutoNumbers = @(
            $document.SelectNodes("//*[local-name()='AUTONUM']") |
                Where-Object { $_.GetAttribute('NumberType') -eq 'Figure' }
        ).Count
        Styles = $reportedStyles
        StyleUsage = $reportedStyleUsage
        InterestingRoots = $reportedInterestingRoots
        TargetRoots = $targetRoots
        TargetTables = $targetTables
        TargetStyles = $targetStyles
        FieldLikeElements = $fieldLike
        ElementCounts = $reportedElementCounts
        TemplateUnchanged = ((Get-Sha256 $absoluteFile) -eq $hashBefore)
    } | ConvertTo-Json -Depth 8
}
finally {
    if ($null -ne $hwp) {
        if ($opened) {
            try { $null = $hwp.Clear(1) } catch {}
        }
        try { $null = $hwp.Quit() } catch {}
        if ([Runtime.InteropServices.Marshal]::IsComObject($hwp)) {
            try { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($hwp) } catch {}
        }
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
    [GC]::Collect()
    if ((Get-Sha256 $absoluteFile) -ne $hashBefore) {
        throw 'Inspection changed the source template.'
    }
}
