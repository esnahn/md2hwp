# C# Hancom IR preview investigation

This is an investigation program, not the production Hancom Automation
backend. It shows how validated IR plus a closed investigation template profile
can drive a safe edit of an HWP copy before the production lowering contract is
implemented.

The separate [explicit-range declaration experiment](../../../../docs/design/template-declarations-investigation.md)
defines insertion points, paragraph samples, paired prototype boundaries, and
owner-scoped slots. `TemplateDeclarations.cs` is a COM-free lexical validator
with a separate fixture harness. Only the dedicated `investigate-ranges` mode
consumes it; the render mode still requires its investigation profile.

It has eight modes:

- `author-tagged` authors explicit sample ranges, style declarations, and
  prototype slots in a copy of the exact original minimal fixture.
- `render-tagged` consumes IR plus a `minimal-1` tagged HWP, with no external
  profile. It binds actual native sample styles, preserves same-paragraph line
  breaks, clones box/figure structures, removes the bounded sample area, and
  verifies the saved/reopened result. See the
  [tagged template guide](../../../../docs/development/minimal-tagged-template.md)
  for the precise contract, commands, tested coverage, and limits.

- `investigate-ranges` performs the profile-free, minimal-fixture-only
  [native paragraph-range experiment](../../../../docs/development/template-range-investigation.md).
  It saves/reopens an authored copy, captures two sample paragraphs, deletes
  explicit ranges, clones them at the declared target, and checks preservation.
  Its diagnostic output deliberately retains copied tag text; it is not a
  manuscript renderer.

- `plan` parses the closed IR v0.1 shape without COM and emits the exact preview
  operations and selected profile ID as JSON;
- `probe` registers the security module, opens an HWP without saving, closes
  it, and verifies that the input hash did not change;
- `render` copies an HWP to a temporary sibling, binds supported symbolic
  paragraph styles to unique AURI native style names, appends text, cloned box
  structures, native bullet/numbered lists, and repository-local PNG figures
  from IR, saves and reopens it,
  verifies the appended paragraph order, text, paragraph styles,
  character-mark runs, list definitions, box and automatic-number caption
  structures, and picture count, then publishes the requested output path.
- `prepare-template-pair` makes a byte-identical baseline copy and a separately
  saved working copy with one dedicated
  `{{MD2HWP_INSERTION_TARGET_V0_1}}` paragraph at the document end. It reopens
  the working copy and requires exactly one simple marker paragraph while
  preserving the source hash;
- `export-images` opens an HWP read-only, uses Hancom's PNG `SaveAs` support to
  render every page into a new output directory, validates every PNG header and
  records its dimensions and hash, and verifies that the HWP did not change.

The whole-document PNG call follows Hancom's documented Automation example:
[`SaveAs(path, "PNG", "")`](https://forum.developer.hancom.com/t/createpageimage/1861).

The preview currently binds `body`, headings 1 through 6, `block.box`, figure
anchors, captions, and source lines by exact native style name. Nested
`strong`/`emph` nodes are lowered to character-shape runs, including marks in a
link label; the link target and title are intentionally not rendered by this
AURI preview. List paragraphs retain typed marker data and render with Hancom's
native bullet or paragraph-numbering feature over the bound `body` style; no
literal marker is inserted. A `verbatim_block` is lowered to one typed
`block.box` plan operation and,
during render, clones the uniquely matched box prototype from
`tests/fixtures/templates/minimal.hwp`. Its logical lines remain inside one
native box paragraph as HWP line breaks, including empty and trailing lines. IR
line breaks outside a verbatim block in the legacy `render` mode are still
previewed as separate HWP paragraphs; `render-tagged` preserves them within
their paragraph. Legacy limitations are present in `plan` output and must not be
copied into production lowering.

The box selector is deliberately fixture-specific. It requires one root `본문`
paragraph containing one inline table with exactly two internal paragraphs:
one `박스내용` content paragraph and one `출처 및 하단설명` placeholder. The
source placeholder remains template decoration because verbatim-block IR has no
source metadata. The full AURI reference template has a different root style
and four internal `박스내용` paragraphs, so this selector fails preflight there
instead of guessing. A production profile must define that separate structure
and the source-line policy first.

Figure captions use a second fixture-specific prototype. The preview requires
exactly one root `표그림_캡션` paragraph whose direct text content is
`[그림 `, one native `AUTONUM` with `NumberType=Figure` and decimal formatting,
then `] 스타일 대응 예시`. It captures that paragraph as a native HWP
`saveblock`, inserts the control without the clipboard, and replaces only the
human caption suffix. The literal label and the automatic-number control remain
native; nested IR strong/emphasis marks are applied only to the replacement
suffix. The full AURI reference document instead nests the observed caption in
a picture object, so this minimal-root selector rejects it. Production lowering
needs a separate profile-backed selector for that structure.

## Setup and non-COM verification

From PowerShell 7:

```powershell
pwsh -NoProfile -File .\dependencies\install-dotnet-sdk.ps1
pwsh -NoProfile -File .\tools\development\dotnet.ps1 build `
  .\tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj `
  --configuration Release
pwsh -NoProfile -File .\tools\smoke\test-hancom-ir-preview-plan.ps1
```

The SDK is the locked repository-local .NET 10.0.400 Windows x64 SDK. The
project uses no external NuGet package and its restore configuration does not
read user-level NuGet settings.

## Manual COM probe and render

Do not run COM modes from the default Codex sandbox, PowerShell 7, a service,
or a noninteractive session. Use the verified Windows PowerShell 5.1 x64 STA
context and run the open-only mode first in a fresh process:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode probe `
  -Template .\tests\fixtures\templates\minimal.hwp
```

The C# child process on the reference workstation was adopted after this probe
and a complete initial render passed on 2026-08-28. A different workstation
or Windows identity must establish its own result in
`docs/development/environment.md` before relying on `render`. The style-aware
edit command is:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode render `
  -Ir .\examples\ir-v0.1.json `
  -Profile .\profiles\templates\auri-basic\investigation-v0.1.json `
  -Template .\tests\fixtures\templates\minimal.hwp `
  -Output .\artifacts\csharp-ir-box-preview.hwp `
  -Visible
```

The visible render above is for a human-run diagnostic only.
An automated invocation can omit `-Visible` and keep HWP hidden.
The profile is required by `plan` and `render`. Its exact fixture byte length
and SHA-256 are checked before COM starts, and its closed values supply native
style names, reset style, prototype selectors, figure width/source label, and
list depth/indentation policy. The source template is never opened for writing.
Existing output is rejected,
and failed rendering leaves neither the requested output nor a temporary copy.
Before editing, the preview requires exactly one paragraph style with each
expected native name. After reopening, it checks the newly appended paragraph
sequence rather than accepting matching text elsewhere in the template. It
also reads the reopened HWPML character-shape definitions and requires every
text run to have the effective marks `native style base OR IR semantic mark`.
HWP 2020 returned a COM object from `InsertPicture` in the adopted .NET
late-binding context; the preview accepts that result but still requires the
reopened picture count to increase by the exact expected amount.

## Tracked minimal comparison pair

The normal visual comparison starts with the two repository-owned fixtures:

- `tests/fixtures/templates/minimal.hwp` is the untouched baseline;
- `tests/fixtures/templates/minimal-marker.hwp` contains one guarded marker at
  the document end and is bound by
  `profiles/templates/auri-basic/minimal-marker-investigation-v0.1.json`.

This small pair is the current runtime basis; the full AURI report remains
structural reference material. Visual equivalence of the current double-curly-brace
marker fixture has not yet been verified.

## Prepare a minimal-template pair

The intended working template will be based on `tests/fixtures/templates/minimal.hwp`.
Prepare a baseline and a marked copy of that fixture under ignored `artifacts/`:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode prepare-template-pair `
  -Template .\tests\fixtures\templates\minimal.hwp `
  -Baseline .\artifacts\minimal-baseline.hwp `
  -Marked .\artifacts\minimal-marked.hwp
```

All three paths must be distinct, both output paths must be new, and their
parent directories must already exist. The baseline is byte-identical to the
source. The marked copy retains the fixture content and adds one dedicated
`{{MD2HWP_INSERTION_TARGET_V0_1}}` paragraph at the document end.
The source must not already contain the marker. After save/reopen, the marker
must occur in exactly one dedicated root paragraph without tables, pictures,
or automatic-number controls. Temporary files and partially published outputs
are cleaned up on failure.

This mode prepares the pair only; it does not insert IR content at the marker.
The `investigation-v0.1.json` profile identifies the unmarked `minimal.hwp`
fixture. The separate
`minimal-marker-investigation-v0.1.json` profile identifies the tracked
`minimal-marker.hwp` and supports its marker followed by an empty terminal
paragraph. A newly generated copy or user-authored template needs its own
verified identity before rendering.
Hancom may rewrite native metadata when saving; generated HWP byte equality
is not required. Visual equivalence outside the marker must be checked
separately. The full AURI reference is structural reference material and is
not required for this workflow.

The repository-owned `minimal-marker.hwp` fixture and
`minimal-marker-investigation-v0.1.json` profile exercise the same selector with
the complete IR example. The 2026-09-08 live render removed the marker, added
one box, one picture, one Figure automatic number, and three native-list
paragraphs, and passed every reopened structural/style/text check. PNG visual
equivalence with the document-end profile render has not been verified for
the current double-curly marker fixture.

For each `block.box`, the preview selects the uniquely bound template root,
captures a native HWP `saveblock` in memory, and inserts it at the document end
with `SetTextFile(..., "HWP", "insertfile")`; it does not use the system
clipboard. The inserted clone must add exactly one table and no picture or
automatic-number control. The source prototype must remain unchanged, and the
saved clone must retain the expected root/body/table/content/source structure
and exact logical content. HWP regenerates native `InstId` and shape `ZOrder`
values during insertion. It also recalculates the box-layout `LastWidth` and
`SIZE.Height` as logical lines change. Verification excludes those
four known instance/layout fields, normalizes only the box content payload for
the final structure comparison, and compares the remaining prototype XML.
Direct HWPML2X insertion was rejected after it produced no inserted control in
this environment.

The caption path uses the same native `saveblock`/`insertfile` mechanism. HWP
2020 serializes the selected caption's HWPML inspection form inside the
section-definition `TEXT`, not as the ordinary root paragraph seen in the full
document, so preflight validates the exact `CHAR` siblings around its unique
Figure `AUTONUM`. After native insertion, the ordinary root paragraph must be
an exact structural clone except for the regenerated automatic number and
replacement suffix. `InsertFile` also materializes the preceding picture's
previously-zero derived rotation centre; root-sequence comparison excludes only
that lazy `CenterX`/`CenterY` pair while retaining the picture geometry and
transform checks.

The character-mark implementation deliberately uses the dedicated
`CharShapeBold` and `CharShapeItalic` transitions while inserting a paragraph,
with the base state taken from the bound style's HWPML character shape. In the
reference HWP 2020 process, `GetDefault("CharShape", ...)` returned a zeroed
full character-shape set at the insertion caret. Executing that set produced
correct bold/italic flags but also zero height, font, ratio, and relative-size
values, so it must not be used as a partial patch.

For visual review, export all pages from a completed preview into a directory
that does not already exist:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode export-images `
  -Document .\artifacts\csharp-ir-box-preview.hwp `
  -Output .\artifacts\csharp-ir-box-preview-pages
```

The style-aware reference-workstation export was checked again on 2026-08-30.
Hancom 2020 produced `page001.png` through `page003.png`, each 992 by 1403 pixels, and
left the HWP unchanged. The images confirmed that the template cover and page
decorations survived, the heading/body/caption/source typography came from the
template's native styles, nested bold/italic marks were visible without losing
the base font or size, and the 142 mm figure retained its 3:2 ratio. The reopened
HWPML run check passed as well. A later render cloned the minimal fixture's
native box control: the new `block.box` showed the same border and internal
style, retained its empty line, and passed the reopened table/content check.
An additional two-box render used prototype-identical content to exercise the
otherwise ambiguous adjacent-root case and reported `BoxesAdded=2`. Both clones
reopened with exact logical lines, and a two-page PNG export showed both native
boxes with content-dependent heights. The template hash remained unchanged.
An additional two-figure render used captions identical to the source prototype
to exercise adjacent-root ambiguity. It reported `CaptionsAdded=2`, reopened
with two new Figure `AUTONUM` controls, and the PNG pages showed `[그림 1]` and
`[그림 2]`. A later native-list render added two bullet items and a nested
ordered item starting at `3`. The template's current font also renders the city
emoji as missing-glyph boxes; a separate save/reopen probe confirmed that this
is a rendering limitation in the tested path, not loss of the underlying
`U+1F3D9 U+FE0F` values. Generated page images stay under ignored `artifacts/`;
they are evidence for human review, not fixtures.

The native-list path follows the action and `ParaShape` fields documented in
Hancom's local [action table](../../../../reference/hwpautomation/ActionTable_2504.pdf)
and [parameter-set table](../../../../reference/hwpautomation/ParameterSetTable_2504.pdf).
The plan preserves list identity, kind, depth, start, current number, and
segment start. Rendering applies `PutBullet` for bullets. Ordered segments use
`PutParaNumber`, then `ParagraphShape` with `HeadingType=Number`, the IR depth,
`Numbering.NewList=1`, both the root and current-level start number, and decimal
number format. The tested `PutNewParaNumber` path reset the requested start to
1, so it is not used. Save/reopen verification requires one native list
paragraph per IR item, stable definition identity across adjacent items, the
requested ordered start, and the expected paragraph level and margin.

For this investigation preview only, every ordered depth displays decimal
digits followed by a period, and each depth adds 2,000 HWPUNIT (about 7.06 mm)
to the bound `body` left margin. The display sequence and indentation must move
to a production template profile before backend adoption. HWP 2020 displayed a
modal style-overwrite question when `StyleEx("본문")` was redundantly applied to
the directly indented list successor. The adopted transition uniquely binds
the fixture's `바탕글` reset style, clears the native heading state, restores the
bound body margin, then applies `바탕글` before returning to `본문`. This follows
the observed HWP behavior that visiting a different style suppresses the
same-style overwrite question; it never accepts that prompt. The preview
currently rejects list depth above 6 and a `line_break` inside the
marker-bearing paragraph rather than guessing a lowering.
