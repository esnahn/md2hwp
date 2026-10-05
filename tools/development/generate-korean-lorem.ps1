#requires -Version 7.4
<#
.SYNOPSIS
Print Korean lorem ipsum using the preceding word's last Unicode character.
.DESCRIPTION
Accepts only a positive word count and writes that many whitespace-separated
lexical words to stdout, then exits. Punctuation does not count as a word.
Paragraphs break at the first sentence end at or beyond 80 words. Saved
5th/95th percentiles of source sentence lengths control when punctuation becomes
eligible/mandatory.
These use nearest-rank percentiles, counting nonempty paragraph tails as sentences
for body bullet prose without an explicit terminator. Between the limits, tokens
are sampled by source occurrence weight.

Candidate data: examples/korean-lorem/last-character-words.json.
Derived from 402 main-body prose paragraphs from all four unique reports in
reference/auri. The local source corpus is reference/auri/auri-body.txt. Identical basic-report copies are
counted once. Covers, summaries, contents, headings, tables, captions, notes,
references and appendices are excluded. Policy/current-issue body prose uses
native bullet styles rather than the style named 본문.
HWP controls, repeated whitespace and zero-width extraction artifacts are removed.
Punctuation is stored as separate tokens. Decimal/date/URL and abbreviation dots
do not mark sentence boundaries.

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

function Expand-TokenCounts {
    param([object[]]$Counts)
    $pool = [Collections.Generic.List[string]]::new()
    foreach ($entry in $Counts) {
        if ($entry.Count -ne 2) { throw 'A token candidate must contain a token and its count.' }
        $word = [string]$entry[0]
        $frequency = [int]$entry[1]
        if ([string]::IsNullOrWhiteSpace($word) -or [regex]::IsMatch($word, '\s') -or $frequency -lt 1) {
            throw 'Token candidates require a nonempty token and a positive count.'
        }
        for ($repeat = 0; $repeat -lt $frequency; $repeat++) { $pool.Add($word) }
    }
    if ($pool.Count -eq 0) { throw 'Token candidate lists must not be empty.' }
    return ,$pool
}

$punctuation = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$punctuation.UnionWith([string[]]@('.', ',', ';', ':', '!', '?', '。', '！', '？'))
$sentenceEnds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$sentenceEnds.UnionWith([string[]]@('.', '!', '?', '。', '！', '？'))

$modelPath = Join-Path $PSScriptRoot '../../examples/korean-lorem/last-character-words.json'
$strictUtf8 = [Text.UTF8Encoding]::new($false, $true)
$savedModel = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($modelPath, $strictUtf8)) -AsHashtable
$minimumSentenceWords = [int]$savedModel['sentenceLengths']['minimum']
$maximumSentenceWords = [int]$savedModel['sentenceLengths']['maximum']
if ($minimumSentenceWords -lt 1 -or $maximumSentenceWords -lt $minimumSentenceWords) {
    throw 'Sentence word limits must be positive and ordered.'
}
$starts = Expand-TokenCounts $savedModel['starts']
$terminators = Expand-TokenCounts $savedModel['terminators']
$transitions = [Collections.Generic.Dictionary[string, Collections.Generic.List[string]]]::new([StringComparer]::Ordinal)
$wordTransitions = [Collections.Generic.Dictionary[string, Collections.Generic.List[string]]]::new([StringComparer]::Ordinal)
$endingTransitions = [Collections.Generic.Dictionary[string, Collections.Generic.List[string]]]::new([StringComparer]::Ordinal)
foreach ($key in $savedModel['transitions'].Keys) {
    $pool = Expand-TokenCounts $savedModel['transitions'][$key]
    $words = [Collections.Generic.List[string]]::new()
    $endings = [Collections.Generic.List[string]]::new()
    foreach ($token in $pool) {
        if (-not $punctuation.Contains($token)) { $words.Add($token) }
        if ($sentenceEnds.Contains($token)) { $endings.Add($token) }
    }
    $transitions.Add([string]$key, $pool)
    $wordTransitions.Add([string]$key, $words)
    $endingTransitions.Add([string]$key, $endings)
}
foreach ($token in $starts) {
    if ($punctuation.Contains($token)) { throw 'Starting candidates must be lexical words.' }
}
foreach ($token in $terminators) {
    if (-not $sentenceEnds.Contains($token)) { throw 'Terminator candidates must be sentence-ending punctuation.' }
}

$random = [Random]::new()
function Select-NextWord {
    param([string]$LastCharacter)
    if ($wordTransitions.ContainsKey($LastCharacter) -and $wordTransitions[$LastCharacter].Count -gt 0) {
        $pool = $wordTransitions[$LastCharacter]
        return $pool[$random.Next($pool.Count)]
    }
    return $starts[$random.Next($starts.Count)]
}
function Select-NextToken {
    param([string]$LastCharacter, [int]$SentenceLength)
    if ($SentenceLength -ge $maximumSentenceWords) {
        if ($endingTransitions.ContainsKey($LastCharacter) -and $endingTransitions[$LastCharacter].Count -gt 0) {
            $pool = $endingTransitions[$LastCharacter]
        } else { $pool = $terminators }
        return $pool[$random.Next($pool.Count)]
    }
    if ($SentenceLength -lt $minimumSentenceWords) {
        return Select-NextWord $LastCharacter
    }
    if ($transitions.ContainsKey($LastCharacter)) {
        $pool = $transitions[$LastCharacter]
        return $pool[$random.Next($pool.Count)]
    }
    return Select-NextWord $LastCharacter
}

$previousToken = $null
$pendingWord = $null
$sentenceLength = 0
$outputWords = [Collections.Generic.List[string]]::new()
for ($index = 0; $index -lt $WordCount; $index++) {
    if ($null -ne $pendingWord) {
        $word = $pendingWord
        $pendingWord = $null
    } elseif ($null -ne $previousToken) {
        # After punctuation choose only words, avoiding repeated punctuation.
        $word = Select-NextWord (Get-LastCharacter $previousToken)
    } else {
        $word = $starts[$random.Next($starts.Count)]
    }
    $sentenceLength++
    $outputWords.Add([regex]::Replace($word, '[\\`*_{}\[\]<>#!|]', '\$0'))

    # Resolve punctuation after the word before flushing its paragraph or stopping.
    $nextToken = Select-NextToken (Get-LastCharacter $word) $sentenceLength
    if ($punctuation.Contains($nextToken)) {
        $outputWords[$outputWords.Count - 1] += $nextToken
        $previousToken = $nextToken
        if ($sentenceEnds.Contains($nextToken)) { $sentenceLength = 0 }
    } else {
        $pendingWord = $nextToken
        $previousToken = $word
    }

    if (($outputWords.Count -ge 80 -and $sentenceEnds.Contains($nextToken)) -or $index -eq $WordCount - 1) {
        Write-Output ([string]::Join(' ', $outputWords))
        $outputWords.Clear()
        if ($index -lt $WordCount - 1) { Write-Output '' }
    }
}
