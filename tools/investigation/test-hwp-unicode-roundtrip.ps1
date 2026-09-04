# HWP 2020 Unicode normalization investigation. This is not a production backend.
# Findings: docs/development/hwp-unicode-roundtrip.md
[CmdletBinding()]
param(
    [string]$Template = "tests\fixtures\templates\minimal.hwp",
    [string]$Output = "artifacts\hwp-unicode-roundtrip.hwp"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$requiredHostProcess = 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
$requiredInvocation = "$requiredHostProcess -NoProfile -ExecutionPolicy Bypass -Sta -File .\tools\investigation\test-hwp-unicode-roundtrip.ps1"
$actualHostProcess = [IO.Path]::GetFullPath(
    [Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
)
if ($PSVersionTable.PSEdition -ne "Desktop" -or
    $PSVersionTable.PSVersion.Major -ne 5 -or
    $PSVersionTable.PSVersion.Minor -ne 1 -or
    -not [Environment]::Is64BitProcess -or
    [Threading.Thread]::CurrentThread.ApartmentState -ne [Threading.ApartmentState]::STA -or
    -not [Environment]::UserInteractive -or
    [Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0 -or
    -not $actualHostProcess.Equals(
        $requiredHostProcess,
        [StringComparison]::OrdinalIgnoreCase
    )) {
    throw "Run the Unicode round-trip probe in the verified interactive Windows PowerShell 5.1 x64 STA environment: $requiredInvocation"
}

function Resolve-RepositoryPath([string]$Path, [string]$RepositoryRoot) {
    if ([IO.Path]::IsPathRooted($Path)) {
        return [IO.Path]::GetFullPath($Path)
    }
    return [IO.Path]::GetFullPath((Join-Path $RepositoryRoot $Path))
}

function Insert-HwpText($Hwp, [string]$Text) {
    $null = $Hwp.HAction.GetDefault(
        "InsertText",
        $Hwp.HParameterSet.HInsertText.HSet
    )
    $Hwp.HParameterSet.HInsertText.Text = $Text
    if (-not $Hwp.HAction.Execute(
            "InsertText",
            $Hwp.HParameterSet.HInsertText.HSet
        )) {
        throw "HWP failed to insert the Unicode probe text."
    }
}

function Get-DelimitedText(
    [string]$Text,
    [string]$BeginMarker,
    [string]$EndMarker
) {
    $valueStart = $Text.IndexOf($BeginMarker, [StringComparison]::Ordinal)
    if ($valueStart -lt 0) { throw "Missing marker after HWP extraction: $BeginMarker" }
    $valueStart += $BeginMarker.Length
    $valueEnd = $Text.IndexOf(
        $EndMarker,
        $valueStart,
        [StringComparison]::Ordinal
    )
    if ($valueEnd -lt 0) { throw "Missing marker after HWP extraction: $EndMarker" }
    return $Text.Substring($valueStart, $valueEnd - $valueStart)
}

function Format-CodePoints([string]$Text) {
    $formatted = @()
    for ($index = 0; $index -lt $Text.Length; $index++) {
        $first = [int]$Text[$index]
        if ($first -ge 0xD800 -and $first -le 0xDBFF -and
            $index + 1 -lt $Text.Length) {
            $second = [int]$Text[$index + 1]
            if ($second -ge 0xDC00 -and $second -le 0xDFFF) {
                $scalar = 0x10000 + (($first - 0xD800) * 0x400) +
                    ($second - 0xDC00)
                $formatted += "U+{0:X}" -f $scalar
                $index++
                continue
            }
        }
        $formatted += "U+{0:X4}" -f $first
    }
    return $formatted -join " "
}

function Get-HwpPlainText($Hwp) {
    $text = [string]$Hwp.GetTextFile("TEXT", "")
    if ([string]::IsNullOrEmpty($text)) {
        throw "HWP returned no text from GetTextFile."
    }
    return $text
}

function Decode-HwpTextValue([string]$Text) {
    # HWP TEXT transport emits unsupported characters as decimal references.
    return [Net.WebUtility]::HtmlDecode($Text)
}

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$templatePath = Resolve-RepositoryPath $Template $repositoryRoot
$outputPath = Resolve-RepositoryPath $Output $repositoryRoot
if (-not (Test-Path -LiteralPath $templatePath -PathType Leaf)) {
    throw "Missing HWP template: $templatePath"
}
if ($templatePath.Equals($outputPath, [StringComparison]::OrdinalIgnoreCase)) {
    throw "The Unicode probe output must not overwrite its source template."
}
if ([IO.Path]::GetExtension($templatePath) -ine ".hwp" -or
    [IO.Path]::GetExtension($outputPath) -ine ".hwp") {
    throw "The HWP round-trip probe requires HWP input and HWP output."
}

$outputDirectory = Split-Path -Parent $outputPath
if (-not (Test-Path -LiteralPath $outputDirectory -PathType Container)) {
    throw "Missing output directory: $outputDirectory"
}

$dependencyLockPath = Join-Path $repositoryRoot "dependencies\lock.json"
$dependencyLock = Get-Content -Raw -Encoding UTF8 -LiteralPath $dependencyLockPath |
    ConvertFrom-Json
$securityPins = @(
    $dependencyLock.dependencies |
        Where-Object { $_.name -eq "hancom-automation" } |
        Select-Object -ExpandProperty pins |
        Where-Object { $_.name -eq "file-path-checker-module-example" }
)
if ($securityPins.Count -ne 1) {
    throw "Expected exactly one pinned Hancom file-path checker module."
}
$expectedModuleSha256 = [string]$securityPins[0].sha256

$moduleRegistryPath = "HKCU:\Software\HNC\HwpAutomation\Modules"
$moduleName = "FilePathCheckerModuleExample"
$modulePath = $null
$moduleValueKind = $null
if (Test-Path -LiteralPath $moduleRegistryPath) {
    $moduleRegistryKey = Get-Item -LiteralPath $moduleRegistryPath
    if ($moduleRegistryKey.GetValueNames() -contains $moduleName) {
        $modulePath = [string]$moduleRegistryKey.GetValue(
            $moduleName,
            $null,
            [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
        )
        $moduleValueKind = $moduleRegistryKey.GetValueKind($moduleName)
    }
}
if ($moduleValueKind -ne [Microsoft.Win32.RegistryValueKind]::String -or
    [string]::IsNullOrWhiteSpace($modulePath) -or
    -not [IO.Path]::IsPathRooted($modulePath) -or
    -not (Test-Path -LiteralPath $modulePath -PathType Leaf)) {
    throw "The registered Hancom security module is not a valid absolute REG_SZ file path."
}
$modulePath = [IO.Path]::GetFullPath($modulePath)
if ((Get-FileHash -LiteralPath $modulePath -Algorithm SHA256).Hash -ne
    $expectedModuleSha256) {
    throw "The registered Hancom security module does not match dependencies/lock.json."
}

$preexistingHwp = @(Get-Process -Name "Hwp" -ErrorAction SilentlyContinue)
if ($preexistingHwp.Count -ne 0) {
    throw "Close existing HWP processes before the Unicode round-trip probe: $($preexistingHwp.Id -join ', ')"
}

$templateHashBefore = (Get-FileHash -LiteralPath $templatePath -Algorithm SHA256).Hash
Copy-Item -LiteralPath $templatePath -Destination $outputPath -Force

$nfcExpected = [string][char]0xAC00
$decomposedExpected = [string][char]0x1100 + [string][char]0x1161
$emojiExpected = [string][char]0xD83C +
    [string][char]0xDFD9 +
    [string][char]0xFE0F
$runId = [guid]::NewGuid().ToString("N")
$nfcBegin = "MD2HWP_${runId}_NFC_BEGIN"
$nfcEnd = "MD2HWP_${runId}_NFC_END"
$decomposedBegin = "MD2HWP_${runId}_DECOMPOSED_BEGIN"
$decomposedEnd = "MD2HWP_${runId}_DECOMPOSED_END"
$emojiBegin = "MD2HWP_${runId}_EMOJI_BEGIN"
$emojiEnd = "MD2HWP_${runId}_EMOJI_END"

$hwp = $null
$documentOpen = $false
$primaryFailure = $null
$cleanupFailures = @()
$result = $null
try {
    $hwp = New-Object -ComObject HWPFrame.HwpObject
    # RegisterModule must be the first COM call after object creation.
    if (-not $hwp.RegisterModule("FilePathCheckDLL", $moduleName)) {
        throw "Hancom rejected the registered file-access security module."
    }
    if (-not $hwp.Open(
            $outputPath,
            "HWP",
            "lock:false;forceopen:true;suspendpassword:true;versionwarning:false"
        )) {
        throw "HWP failed to open the Unicode probe copy."
    }
    $documentOpen = $true

    if (-not $hwp.HAction.Run("MoveDocEnd")) {
        throw "HWP failed to move to the document end."
    }
    if (-not $hwp.HAction.Run("BreakPara")) {
        throw "HWP failed to create the Unicode probe paragraph."
    }
    Insert-HwpText $hwp ($nfcBegin + $nfcExpected + $nfcEnd)
    if (-not $hwp.HAction.Run("BreakPara")) {
        throw "HWP failed to separate the Unicode probe values."
    }
    Insert-HwpText $hwp (
        $decomposedBegin + $decomposedExpected + $decomposedEnd
    )
    if (-not $hwp.HAction.Run("BreakPara")) {
        throw "HWP failed to separate the Unicode probe values."
    }
    Insert-HwpText $hwp ($emojiBegin + $emojiExpected + $emojiEnd)

    $beforeSaveText = Get-HwpPlainText $hwp
    $nfcBeforeSaveTransport = Get-DelimitedText (
        $beforeSaveText
    ) $nfcBegin $nfcEnd
    $decomposedBeforeSaveTransport = Get-DelimitedText (
        $beforeSaveText
    ) $decomposedBegin $decomposedEnd
    $emojiBeforeSaveTransport = Get-DelimitedText (
        $beforeSaveText
    ) $emojiBegin $emojiEnd
    $nfcBeforeSave = Decode-HwpTextValue $nfcBeforeSaveTransport
    $decomposedBeforeSave = Decode-HwpTextValue $decomposedBeforeSaveTransport
    $emojiBeforeSave = Decode-HwpTextValue $emojiBeforeSaveTransport

    if (-not $hwp.HAction.Run("FileSave")) {
        throw "HWP failed to save the Unicode probe document."
    }
    $null = $hwp.Clear(1)
    $documentOpen = $false

    if (-not $hwp.Open(
            $outputPath,
            "HWP",
            "lock:false;forceopen:true;suspendpassword:true;versionwarning:false"
        )) {
        throw "HWP failed to reopen the saved Unicode probe document."
    }
    $documentOpen = $true

    $afterReopenText = Get-HwpPlainText $hwp
    $nfcAfterReopenTransport = Get-DelimitedText (
        $afterReopenText
    ) $nfcBegin $nfcEnd
    $decomposedAfterReopenTransport = Get-DelimitedText (
        $afterReopenText
    ) $decomposedBegin $decomposedEnd
    $emojiAfterReopenTransport = Get-DelimitedText (
        $afterReopenText
    ) $emojiBegin $emojiEnd
    $nfcAfterReopen = Decode-HwpTextValue $nfcAfterReopenTransport
    $decomposedAfterReopen = Decode-HwpTextValue $decomposedAfterReopenTransport
    $emojiAfterReopen = Decode-HwpTextValue $emojiAfterReopenTransport

    $nfcPreserved = [string]::Equals(
        $nfcExpected,
        $nfcAfterReopen,
        [StringComparison]::Ordinal
    )
    $decomposedPreserved = [string]::Equals(
        $decomposedExpected,
        $decomposedAfterReopen,
        [StringComparison]::Ordinal
    )
    $emojiPreserved = [string]::Equals(
        $emojiExpected,
        $emojiAfterReopen,
        [StringComparison]::Ordinal
    )

    $result = [pscustomobject]@{
        Output = $outputPath
        NfcExpected = Format-CodePoints $nfcExpected
        NfcBeforeSaveTransport = $nfcBeforeSaveTransport
        NfcBeforeSave = Format-CodePoints $nfcBeforeSave
        NfcAfterReopenTransport = $nfcAfterReopenTransport
        NfcAfterReopen = Format-CodePoints $nfcAfterReopen
        NfcPreserved = $nfcPreserved
        DecomposedExpected = Format-CodePoints $decomposedExpected
        DecomposedBeforeSaveTransport = $decomposedBeforeSaveTransport
        DecomposedBeforeSave = Format-CodePoints $decomposedBeforeSave
        DecomposedAfterReopenTransport = $decomposedAfterReopenTransport
        DecomposedAfterReopen = Format-CodePoints $decomposedAfterReopen
        DecomposedPreserved = $decomposedPreserved
        EmojiExpected = Format-CodePoints $emojiExpected
        EmojiBeforeSaveTransport = $emojiBeforeSaveTransport
        EmojiBeforeSave = Format-CodePoints $emojiBeforeSave
        EmojiAfterReopenTransport = $emojiAfterReopenTransport
        EmojiAfterReopen = Format-CodePoints $emojiAfterReopen
        EmojiPreserved = $emojiPreserved
    }

    if (-not $nfcPreserved -or -not $decomposedPreserved -or
        -not $emojiPreserved) {
        throw "HWP changed Unicode scalar values during the save/reopen round trip. NFC=$nfcPreserved; decomposed=$decomposedPreserved; emoji=$emojiPreserved; emoji expected=$($result.EmojiExpected); emoji actual=$($result.EmojiAfterReopen)"
    }
}
catch {
    $primaryFailure = $_
}
finally {
    if ($null -ne $hwp) {
        if ($documentOpen) {
            try { $null = $hwp.Clear(1) }
            catch { $cleanupFailures += "document close: $($_.Exception.Message)" }
        }
        try { $null = $hwp.Quit() }
        catch { $cleanupFailures += "Quit: $($_.Exception.Message)" }
        if ([Runtime.InteropServices.Marshal]::IsComObject($hwp)) {
            try { $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($hwp) }
            catch { $cleanupFailures += "COM release: $($_.Exception.Message)" }
        }
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()

    $shutdownDeadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $remainingHwp = @(Get-Process -Name "Hwp" -ErrorAction SilentlyContinue)
        if ($remainingHwp.Count -eq 0) { break }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $shutdownDeadline)

    if ($remainingHwp.Count -ne 0) {
        $cleanupFailures += "HWP process(es) remained after Quit and COM release: $($remainingHwp.Id -join ', '). No process was force-stopped."
    }
}

$templateHashAfter = (Get-FileHash -LiteralPath $templatePath -Algorithm SHA256).Hash
if ($templateHashAfter -ne $templateHashBefore) {
    throw "The source HWP template changed during the Unicode probe."
}
if ($null -ne $primaryFailure) {
    if ($cleanupFailures.Count -ne 0) {
        throw "$($primaryFailure.Exception.Message) Cleanup also failed: $($cleanupFailures -join '; ')"
    }
    throw $primaryFailure
}
if ($cleanupFailures.Count -ne 0) {
    throw "Unicode probe cleanup failed: $($cleanupFailures -join '; ')"
}

$result
