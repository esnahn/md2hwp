#requires -Version 5.1

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$File,

    [string]$RootIndexes = ''
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

    [object[]]$reportedStyles = @()
    [object[]]$reportedStyleUsage = @()
    [object[]]$reportedInterestingRoots = @()
    [object[]]$reportedElementCounts = @()
    if ($requestedRootIndexes.Count -eq 0) {
        $reportedStyles = @(
            $styles.GetEnumerator() |
                Sort-Object Name |
                ForEach-Object {
                    [pscustomobject]@{ Id = [int]$_.Key; Name = [string]$_.Value }
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
