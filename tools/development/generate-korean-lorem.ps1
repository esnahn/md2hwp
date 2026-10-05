#requires -Version 7.4
<#
.SYNOPSIS
Print Korean lorem ipsum using the preceding word's last Unicode character.
.DESCRIPTION
Accepts only a positive word count and writes that many whitespace-separated
words to stdout, then exits. Blank lines every 80 words form Markdown paragraphs.

Candidate data: examples/korean-lorem/last-character-words.json.
Derived from 402 main-body prose paragraphs from all four unique reports in
reference/auri. The local source corpus is reference/auri/auri-body.txt. Identical basic-report copies are
counted once. Covers, summaries, contents, headings, tables, captions, notes,
references and appendices are excluded. Policy/current-issue body prose uses
native bullet styles rather than the style named 본문.
HWP controls, repeated whitespace and zero-width extraction artifacts are removed.

Original HWP SHA-256 values:
01 기본: 0BA84133775B182C76ACE779082ED77E4BCE0743C00733FB6C7AF7CD43B3BAF1
02 정책: B268B9D3286675696769ED282467DE27D9C59BCD7A6B4FB120179F580F5C6171
03 일반: DD7AD35EA0FB6BA5D353926998CA12FDCD59248E68B009A26D28D0C3FA34DA65
04 현안: 3D02EA4B6CB264BB569142C62133622F5E8A16858D7F42F7F2FCB887E86409A8

Words retain source occurrence weights. The last written character, such as
환경을 -> 을, selects the next word; it is not a Hangul final-consonant model.
The saved word/count tables omit prose and paragraph order. Generation reads
only candidate data; it needs no Hancom or local source corpus.
.EXAMPLE
pwsh -NoProfile -File tools/development/generate-korean-lorem.ps1 10000
#>
param([int]$WordCount)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $PSBoundParameters.ContainsKey('WordCount') -or $WordCount -lt 1 -or $args.Count -gt 0) {
    [Console]::Error.WriteLine('Usage: pwsh -NoProfile -File tools/development/generate-korean-lorem.ps1 <positive-word-count>')
    exit 2
}

function Get-LastCharacter {
    param([string]$Word)
    $characterIndex = $Word.Length - 1
    if ($characterIndex -gt 0 -and [char]::IsSurrogatePair($Word, ($characterIndex - 1))) {
        $characterIndex--
    }
    $Word.Substring($characterIndex)
}

function Expand-WordCounts {
    param([object[]]$Counts)
    $pool = [Collections.Generic.List[string]]::new()
    foreach ($entry in $Counts) {
        if ($entry.Count -ne 2) { throw 'A word candidate must contain a word and its count.' }
        $word = [string]$entry[0]
        $frequency = [int]$entry[1]
        if ([string]::IsNullOrWhiteSpace($word) -or [regex]::IsMatch($word, '\s') -or $frequency -lt 1) {
            throw 'Word candidates require a nonempty word and a positive count.'
        }
        for ($repeat = 0; $repeat -lt $frequency; $repeat++) { $pool.Add($word) }
    }
    if ($pool.Count -eq 0) { throw 'Word candidate lists must not be empty.' }
    return ,$pool
}

$modelPath = Join-Path $PSScriptRoot '../../examples/korean-lorem/last-character-words.json'
$strictUtf8 = [Text.UTF8Encoding]::new($false, $true)
$savedModel = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($modelPath, $strictUtf8)) -AsHashtable
$starts = Expand-WordCounts $savedModel['starts']
$transitions = [Collections.Generic.Dictionary[string, Collections.Generic.List[string]]]::new([StringComparer]::Ordinal)
foreach ($key in $savedModel['transitions'].Keys) {
    $transitions.Add([string]$key, (Expand-WordCounts $savedModel['transitions'][$key]))
}

$random = [Random]::new()
$previousWord = $null
$outputWords = [Collections.Generic.List[string]]::new()
for ($index = 0; $index -lt $WordCount; $index++) {
    $key = if ($null -ne $previousWord) { Get-LastCharacter $previousWord } else { $null }
    if ($null -ne $key -and $transitions.ContainsKey($key)) {
        $candidates = $transitions[$key]
        $word = $candidates[$random.Next($candidates.Count)]
    } else {
        if ($outputWords.Count -gt 0 -and -not $outputWords[$outputWords.Count - 1].EndsWith('.')) {
            $outputWords[$outputWords.Count - 1] += '.'
        }
        $word = $starts[$random.Next($starts.Count)]
    }
    $previousWord = $word
    $displayWord = [regex]::Replace($word, '[\\`*_{}\[\]<>#!|]', '\$0')
    if (($index + 1) % 16 -eq 0 -or $index -eq $WordCount - 1) { $displayWord += '.' }
    $outputWords.Add($displayWord)
    if ($outputWords.Count -eq 80 -or $index -eq $WordCount - 1) {
        Write-Output ([string]::Join(' ', $outputWords))
        $outputWords.Clear()
        if ($index -lt $WordCount - 1) { Write-Output '' }
    }
}
