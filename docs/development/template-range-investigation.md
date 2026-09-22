# Native paragraph-range experiment, 2026-09-22

Status: investigation evidence only; not a renderer or adopted template syntax.

## Scope and command

`investigate-ranges` requires a byte-identical copy of tracked `minimal.hwp`.
It needs no external profile. It appends a content target with an empty
successor, a Korean guard paragraph, and a bounded samples area containing the
experimental contract declaration and body/heading sample paragraphs.

After saving and reopening the authored copy, it parses those declarations,
selects the two sample paragraphs through the next paragraph beginning, and
captures an HWP `saveblock`. It deletes the complete samples area, resolves the
content target again, deletes that paragraph, and inserts the captured block.
It checks all text immediately before and after each deletion. Both new paths
must be absent; temporary copies are published only after checks and cleanup.

Build with the repository-local SDK:

```powershell
pwsh -NoProfile -File .\tools\development\dotnet.ps1 build .\tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj --configuration Debug
```

Then run in the verified interactive Windows PowerShell 5.1 x64 STA context:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode investigate-ranges `
  -Template .\tests\fixtures\templates\minimal.hwp `
  -Marked .\artifacts\ranges-authored-20260922-v2.hwp `
  -Output .\artifacts\ranges-result-20260922-v2.hwp `
  -Configuration Debug
```

Choose new output names for another run. `-Marked` is the authored diagnostic
template; `-Output` is the diagnostic clone result. Body/heading tag text
deliberately remains visible in the clone to identify its source paragraphs.
No manuscript rendering or final tag removal is claimed.

## Observations

- The authored template and final result both saved and reopened successfully.
- Exactly two sample paragraphs were cloned with their native style IDs.
  Samples inherit the fixture's current style; distinct-role style resolution
  was not exercised.
- The whole samples range and insertion paragraph were removed without losing
  the following paragraph's first character. Full paragraph text sequences
  matched at each checked stage.
- Original paragraph HWPML matched after canonicalizing only relative
  `SHAPEOBJECT/@ZOrder` ranks across the original paragraphs. Table, picture,
  and automatic-number counts were unchanged.
- An initial strict XML comparison caught native ZOrder renumbering, including
  `35 -> 5`, `36 -> 6`, and `32 -> 2`. The comparator preserves relative order
  and ties; it does not discard ZOrder. Regression checks reject reordered
  objects, changed ties, text changes, and paragraph-style changes.
- Source SHA-256 remained
  `5CAEABF6C3BF1EE10B68FF810678374B0C775CB6A27B423F5326F3ED97080D55`.
- Normal COM cleanup completed. Failed strict-comparison runs published neither
  authored nor result output, and the same paths were reusable afterward.

## Visual evidence

Hancom exported the original and final result as two 992 x 1403 PNG pages.
The first-page PNG was byte-identical to the original, SHA-256
`8B12E3BC4909E9201617D853643F789BD7128F3ED67EA0B298BF7C6D5908D56B`.
The second page was visually inspected against the original: existing heading,
body, box, source line, caption, and margin content stayed in place. The two
sample tags and the guard paragraph appeared below the existing content.

Local generated evidence (ignored, reproducible with the command above):

- `artifacts/ranges-authored-20260922-v2.hwp`
- `artifacts/ranges-result-20260922-v2.hwp`
- `artifacts/ranges-baseline-20260922-pages/page001.png` and `page002.png`
- `artifacts/ranges-result-20260922-v2-pages/page001.png` and `page002.png`

Final result page 2 SHA-256:
`4BBA31D05177730B5063645CE72CA6CF70F373D43E412FC4B35AA6E8019BED66`.

## Limits and next investigation

This proves a two-paragraph text range on one fixture in one section. It does
not prove native named-field persistence, multi-section ranges, nested control
ownership, actual box/figure prototype capture, figure caption binding,
native-list inheritance, character-mark preservation, or general document
fidelity. Referenced global style-definition equivalence is not independently
checked by comparing paragraph XML. Native coordinates remain private to this
fixture-specific investigator.

Next compare a named-field range against the paired-marker candidate, then
extend the explicit-boundary fixture to a whole inline box and a figure with
native caption numbering. Keep those as separate reviewable experiments before
adopting a production template contract.
