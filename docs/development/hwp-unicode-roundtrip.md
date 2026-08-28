# HWP Unicode round-trip investigation

## Scope

This is an investigation result for the installed reference environment, not a
general compatibility claim for every Hancom or Unicode version. It was
verified on 2026-08-28 with:

- Hancom Office 2020 HWP `11.0.0.9136`;
- the registered, pinned `FilePathCheckerModuleExample` security module;
- interactive Windows PowerShell 5.1 x64 in an STA;
- the HWP format, not HWPX;
- the tracked fixture at `tests/fixtures/templates/minimal.hwp`.

The repeatable investigation script is
`tools/investigation/test-hwp-unicode-roundtrip.ps1`. Run it from the repository
root with:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\test-hwp-unicode-roundtrip.ps1
```

The script copies the fixture to the ignored
`artifacts/hwp-unicode-roundtrip.hwp`; it never opens the fixture itself for
editing. A different HWP fixture may be supplied with `-Template`, and the
output may be changed with `-Output`. Input and output must be distinct HWP
paths.

## Method

The probe constructs the two test values from numeric UTF-16 code units so the
PowerShell source file cannot pre-normalize them:

```text
NFC:        U+AC00
decomposed: U+1100 U+1161
```

It then:

1. creates `HWPFrame.HwpObject` and immediately requires
   `RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample")`;
2. opens a copy of the fixture and inserts both values between unique ASCII
   markers with `InsertText`;
3. extracts text before saving;
4. saves the HWP, closes it with `Clear(1)`, and reopens the saved path;
5. extracts the marked values again and compares them ordinally with the input;
6. verifies that the source fixture hash did not change and waits for HWP to
   exit without force-stopping any process.

The close and reopen happen through the same `HwpObject`. This proves a file
save/reopen boundary, but not a fresh worker-process boundary. A later backend
conformance test may split writing and verification into separate processes if
that stronger isolation is required.

## Verified result

| Value | Input | Before save | After reopen | Preserved |
|---|---|---|---|---|
| precomposed `가` | `U+AC00` | `U+AC00` | `U+AC00` | yes |
| decomposed `가` | `U+1100 U+1161` | `U+1100 U+1161` | `U+1100 U+1161` | yes |

For these two values, the tested HWP 2020 path did not perform canonical
composition or decomposition during insertion, save, or reopen.

## TEXT extraction transport

`GetTextFile("TEXT", "")` does not expose every character literally. In this
test it returned the precomposed syllable literally, but represented the two
decomposed Jamo as decimal character references both before saving and after
reopening:

```text
가
&#4352;&#4449;
```

Decimal `4352` is `U+1100`, and decimal `4449` is `U+1161`. The probe decodes
the marked value with `[Net.WebUtility]::HtmlDecode` before comparing code
points. Comparing the raw TEXT transport would produce a false normalization
failure even though the original code points remain present.

Backend verification must therefore distinguish:

- the HWP document's Unicode value;
- the TEXT export's escaped transport representation; and
- the decoded string used for ordinal code-point comparison.

Do not call `Normalize()` in the converter or backend unless a later explicit
policy requires it. This result covers only the tested two-value fixture; it
does not prove preservation of every Unicode sequence, every template, HWPX,
or another Hancom release.

## PowerShell source encoding

Windows PowerShell 5.1 misread a Korean path literal when the script was stored
as UTF-8 without a BOM. Repository PowerShell files now use the
`.editorconfig` rule `charset = utf-8-bom`. A script containing Korean paths or
text must retain the UTF-8 BOM; numeric construction remains preferable for
the actual code-point fixtures.
