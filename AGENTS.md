# AGENTS.md

## Repository boundary

The repository was cleaned up while preparing the first release, ultimately
published as v0.2.0. Keep current application/build sources, templates, active
documentation, test fixtures and examples, and maintained dependency scripts.
Historical investigations and obsolete documentation are recoverable from Git
history; do not recreate scaffolding or obsolete investigation workflows.
Keep README.md accurate. Build via tools/development/build.ps1; verify Rust with
cargo test --workspace. The old standalone smoke scripts and canonical COM probe
were removed by this cleanup; do not claim they still exist or have passed.

Rust application: apps/md2hwp. Shared semantic core: crates/md2hwp-core.
C# backend: backends/hancom-automation/Md2Hwp.Backend.csproj.
Template: templates/template.hwp. Build outputs: target/, bin/, obj/ (ignored).
Use target/debug and target/release as the primary paths for current development
builds and execution. target/dist is for versioned deployment archives; do not
redirect ordinary builds there merely to preserve an older release.
The JSON fixtures still tracked are included directly by Rust test compilation.
Builds include a deployment README.md with usable versioned documentation links.
Create release ZIPs with tools/development/build.ps1 -Configuration Release -Package;
include both executables, the tracked default template and README.md. Always replace
Debug/Release deployment templates from templates/template.hwp during builds. Attach only the executable ZIP to GitHub Releases; put its
SHA-256 in the release body. Use GitHub's automatic source downloads and do not attach
separate source ZIPs, release-note copies, checksum files or build manifests.

## Contracts

Keep Pandoc invocation in the app and Pandoc AST handling in the core. Backends
consume validated IR. Preserve Unicode. Reject unsupported constructors explicitly.
Use only the current IR 0.4 closed schema. AST2IR rules target that IR without
an independent rules version. Input IR and template ir-version must match the
program's current IR exactly; reject old versions with regeneration guidance.
Template-owned begin:template/end:template declarations supply all current IR
styles and prototype ranges, including roles unused by the manuscript;
do not restore runtime external profiles. Preserve unrelated template content.
Heading roles and template tags use heading1 through heading6 (no dot between
heading and its level), including begin/end/slot and each.child ranges.
Only begin:template may carry native SECDEF/COLDEF controls. Preserve those
settings outside disposable definitions in the working document, without changing
the source template. Do not extend this exception to other declarations.
Generated links render as formatted labels/plain text; strip automatic hyperlinks
only in generated content. Figures embed PNG/JPG/JPEG images; caption and source belong to the picture's native caption.
The begin:figure/end:figure sample contains one picture with a
two-paragraph native caption. Ignore sample image content/size; inherit caption options.
Native list.bullet/list.ordered sample paragraphs own bullet and per-level numbering
formats; manuscript start numbers and existing depth indentation remain authoritative.
Box template boundaries are begin:code/end:code, and the content formatting role is code.
Keep code.content/code.title/code.source slot names.
Verbatim blocks may carry sources. Expand each raw IR line into a separate box
paragraph using the content sample's full formatting. Preserve spaces, tabs and
body blank lines. Optional slot:code.title precedes the content sample in the same
cell; only when this slot exists, interpret an exact first-line 제목: prefix as
the title, removing the prefix and one following space, then skipping immediately
following blank/whitespace-only lines. With no title slot, preserve all raw lines.
Keep this template-dependent presentation in the backend; do not add an IR title
field or parse Markdown syntax inside the remaining verbatim text.
Markdown input uses commonmark+yaml_metadata_block+footnotes+attributes+implicit_figures+pipe_tables. Reference-style
footnotes are enabled; inline_notes remains disabled. The 제목: and 출처: rules
are md2hwp conventions, not CommonMark or Pandoc extension syntax. Document these
separately in README.md.
IR inline footnote contains nonempty paragraph blocks. Allow notes in document
paragraphs, headings, list paragraphs and table cells, including formatted spans and link labels.
Footnote bodies allow multiple paragraphs and existing inline formatting/links/line
breaks. Reject nested footnotes, nonparagraph footnote blocks, and notes in figure
alt/caption, table captions or box/figure/table sources explicitly. Count footnote blocks, inlines and text
against the existing cumulative resource limits. Preserve native FOOTNOTE and
AUTONUM controls, then verify them after save/reopen. Recapture generated
note paths from their first-body-paragraph native identities after cross-reference
run replacement; only generated OnPage counters may be normalized.
Require one {{md2hwp:footnote}} sample inside template definitions: either the
existing plain root paragraph or one native footnote on a plain root anchor.
The native sample has one body paragraph containing its Footnote AUTONUM before
the tag, with only whitespace/native tabs around the tag; the tag cannot cross
a tab or other control. Preserve native TAB controls and their attributes along
with its control/list options,
number format and number character formatting; the tag's first character and full
paragraph formatting own the first generated body paragraph. An optional plain
{{md2hwp:footnote.next}} sample, either a root paragraph inside definitions
or the second paragraph of the same sample note, owns all subsequent paragraphs.
Its single tag may have literal text, native TAB and AUTONUM affixes, preserving their
formatting, positions and values on every continuation paragraph. Other controls and
page breaks remain unsupported; slots cannot cross controls. Track and normalize only
the first paragraph's Footnote AUTONUM, leaving continuation AUTONUMs intact.
Without it retain the first paragraph formatting
for compatibility. New default templates include this separately editable paragraph.
Apply the number and sample
whitespace/tabs only to the first generated paragraph. Reject duplicate/mixed samples,
additional sample paragraphs and samples in cells, headers, footers, master pages,
endnotes or nested notes. Flatten only the disposable native sample anchor in the
working document; keep the source unchanged. init-template creates a native sample.
The source document SECDEF owns native note numbering, separator and placement
options. Preserve those options and document/section start numbers. Notes cannot be copied
into headers, footers or master pages; fail explicitly for such heading slots.
Never derive options from inline text or hardcoded
fixture coordinates.
Hancom 2020 may export OnPage AUTONUM Number="1" for every note while displaying
1, 2 on each page. Normalize only generated note Number attributes for OnPage;
keep continuous/section numbering and all other structures exact. Verify the
rendered footnote numbers visually; XML counters alone do not establish display numbering.
Existing user deployment templates require explicit adoption
of new declarations or regeneration, even while the development IR remains 0.4.
IDs on headings and figures are document-wide unique Unicode strings without
whitespace, control characters or #. Preserve them in IR; decode internal Link
fragments strictly as UTF-8 URI escapes and resolve by actual target kind, without
requiring sec:/fig: prefixes. Normalize both Para/Image and one-image, one-paragraph
Pandoc Figure inputs. Reject unsupported caption/body structures explicitly.
Use xrefs_number-compatible internal Links for number references; do not parse
raw [@...] strings or enable citations/citeproc/pandoc-crossref for this feature.
Validate source link labels and resource limits before replacing them with symbolic
IR cross_reference {kind: heading_number|figure_number, target: id}.
Require single-paragraph ref.figure.number and ref.heading.number prototypes even
when unused. Their wording and slots define reference labels; ignore prototype character
formatting and inherit the first source reference character's complete inline formatting,
including body/note/cell context and emphasis. In figure references num:heading1 is
the target figure's fixed chapter; the figure-number slot is a native Crossref.
Heading references use a native outline Crossref and GetHeadingString, never a
computed heading number. Missing or ambiguous outline targets are errors; repeated
title copies count only when they carry real Outline paragraph formatting.
Default headings remain plain; users configure native outline numbering when needed.
Do not infer native number settings from ID prefixes or manuscript link labels.
No generated reference hyperlinks. Verify native field target/cache/format and
surrounding structures after save/reopen. Keep unreferenced template fields intact.
Use Action.Execute for sets returned by Action.CreateSet; passing those sets to
HAction.Execute fails COM interface conversion on this Hancom 2020 workstation.
Hancom reuses Crossref FieldId values across distinct fields. Require matching local
begin/end FieldId and unique field InstId; do not require global FieldId uniqueness.
Coalesce adjacent identical-format target-marker runs before HWPML import. Reject
empty native outline display formats without calculating a replacement number.
Table/page and other reference roles remain reserved and unsupported.
HWPX and RST input are deferred.

## Tables

Normalize simple Pandoc Table inputs to IR table {columns, header, rows, caption, source}.
Columns carry default/left/center/right alignment; header and row cells contain inline
arrays, including empty cells. Require one header row, one body, rectangular cells
and ColWidthDefault column widths only. Reject table IDs/attributes, merged cells, row-header
columns, multiple/intermediate headers or bodies, footer rows and nonparagraph or
multiple-paragraph cell blocks explicitly. Table number references remain unsupported.
Cells allow rich text, footnotes and current heading/figure references; caption and
source reject footnotes but allow heading/figure number references. Include rows/cells and every inline/text in
cumulative validation limits.

CommonMark does not support table_captions. Attach standalone Table:/table:/:/표: plus
whitespace and nonempty content before or after an adjacent table, preserving rich
inlines and stripping the prefix. Reject ambiguous ownership, captions on both sides
and native-plus-adjacent duplicates. Attach the following 출처: paragraph, allowing
an intervening below-table caption. Place captions above tables in sample manuscripts.
Keep README.md and docs/tables.md accurate; these caption conventions are compatible
with Pandoc Markdown table_captions, not an enabled CommonMark extension.
The Korean 표: prefix is an md2hwp-specific extension of those conventions.

Require table.width-mm before begin:table/end:table, containing one native table
anchor with three rows and two columns: two slot:table.header cells, two
slot:table.content cells and one slot:table.source cell spanning both columns.
Header/content slots occupy their whole single paragraphs. Left samples own common
formatting and horizontal borders; use both samples for outer and internal vertical
borders. The left sample's left edge and right sample's right edge own the exterior;
repeat their facing edges at interior boundaries. A one-column output combines both
outer edges. Reject old one-column prototypes with explicit adoption/regeneration
guidance without changing IR 0.4. The
last cell has no side/bottom borders or background; its visible top border may
exactly match the content sample's bottom border. Preserve that shared boundary;
for header-only outputs align it to the header sample's bottom border. An absent
source top border never removes the preceding row's bottom border. The cell is
cloned as a merged source row only when source
exists. Require a one-paragraph native CAPTION with slot:table.caption and one Table
AUTONUM; num:heading1 is optional there. Omit the native caption when manuscript
caption is absent. Preserve caption options and native Table numbering/restart controls;
do not add chapter-dependent table counter restarts. Ignore prototype dimensions,
copy its formats, and rebuild rows/columns from IR. Default table prototypes disable
TreatAsChar so Cell page breaks and RepeatHeader work across pages. Default alignment preserves
template paragraph alignment; explicit manuscript alignment overrides it.

Calculate content widths using Windows font measurements of the actual header/body
formats, character size/ratio/spacing and emphasis, plus cell/paragraph margins and
1% text-width allowance. Paragraph margins use URC and require conversion to
HWPUNIT; do not double their width. Require installed Windows TTF/OTF faces and
reject HFT or silent GDI font fallback. Fit minimum/preferred widths within table.width-mm (142mm
in the default template); source/caption are excluded. Fail when minimums do not fit.
Require the width to fit the active section/text column, rejecting unequal-width
multi-column documents. Let content expand generated row heights; normalize only
known generated table/cell heights after import and save/reopen, keeping structure,
widths, native captions/numbering and all surrounding content exact. Structural checks
do not establish clipping safety; inspect representative and long-table rendered pages.


## Metadata and dates

Markdown front matter supports title, subtitle, author, date, publisher and
nonempty top-level md2hwp-<name> keys. Custom values are strings; author also
accepts a nonempty string list, displayed in order with comma-space separators.
Document title metadata is separate from heading 1; substitution does not create
headings. md2hwp-heading1-start is a reserved positive decimal setting (default 1),
validated by Rust and the backend; the IR carries its string value in metadata.

Use {{md2hwp:meta:<key>}} for template substitution. Values are literal text,
not recursively interpreted as tags, and do not create automatic hyperlinks.
Tags may span character runs within one paragraph but must not cross controls;
replacement inherits the first tag character's formatting. Referenced missing
values are errors. Preserve unrelated template content and native structures.

Rust core preserves the interpreted date display text and derives date-meta as
YYYY-MM-DD when recognized. YAML cannot supply date-meta directly. Supported
formats, partial-date defaults and two-digit-year boundaries are specified in
docs/date-metadata.md; never substitute the current date. Unrecognized or invalid
dates retain date and omit date-meta. Rust validates the derived value against
date. The backend validates ISO date-meta and formats it without reparsing date.

Only date-meta accepts fmt, for example
{{md2hwp:meta:date-meta:fmt:%Y년 %-m월 %-d일}}.
Supported directives are %Y, %y, %m, %-m, %d, %-d, %F and %%.
Reject unsupported directives and empty or control-containing format strings.
Keep docs/yaml-variables.md and docs/date-metadata.md consistent with the code.

Verify Rust with cargo test --workspace and
cargo clippy --workspace --all-targets -- -D warnings. Run backend contract tests
through the pinned SDK:
pwsh -NoProfile -File tools/development/dotnet.ps1 run --project tests/backend-contract/Md2Hwp.Backend.Tests.csproj
These contract tests do not establish live Hancom or visual layout correctness.

## Template editing and release continuity

Use templates/template.hwp as the authoritative editable default template. Build
outputs under target/ are disposable: build.ps1 always copies the tracked template
to target/debug/template.hwp or target/release/template.hwp with overwrite enabled.
Do not preserve edits to deployment templates during builds. This supersedes the
previous protection of target/release/template.hwp. Use a separate template copy
and explicit --template for experiments and live verification; never mutate the
source template while rendering. Custom templates to retain belong outside target/.

Align release major/minor versions with the IR version (IR 0.4 corresponds to
v0.4.0). A Git tag alone does not update application/package version metadata or
README version labels; check their consistency when preparing a release.

Track heading1 from md2hwp-heading1-start, incrementing only for heading1.
Use num:heading1 inside heading blocks and native figure/table captions; preserve
its first character formatting, reject unresolved or orphan slots. Figure counters
restart with a native NEWNUM Figure=1 at the first figure of each chapter. Keep
AUTONUM as native controls. Literal # markers remain unchanged without the new slot.
User deployment templates require explicit authorization before adopting new slots.

Fixed-size text boxes can overflow when titles or child-heading lists grow.
Structural save/reopen checks do not detect visual clipping or overlap. Inspect
representative rendered pages; do not claim layout safety from structural checks
alone. Title-table height changes are reported separately from structural loss.

## Dependencies

Pinned Rust and .NET SDK settings remain authoritative. Use the repository-local
.NET SDK through tools/development/dotnet.ps1. dependencies/lock.json is build/dev
metadata, not a runtime version gate. Keep the Hancom module development pin.
Never install/register the Hancom security module or mutate its DLL/HKCU settings.
Users manage it via https://developer.hancom.com/hwpautomation.
Require RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample") to return
true before opening a document. Existing file/registration checks remain mandatory.
Pandoc setup is explicit, uses the build-time preferred release or official stable
fallback, checks the digest, and preserves upstream notices. No runtime lock sidecar.

## Hancom safety and verification

Use documented COM only, not UI clicks/keystrokes. Process one document at a time.
Never modify source templates or manuscripts. Replace generated HWP only after
successful temporary rendering, save/reopen and structural verification. Preserve
the old result on failure. Do not accept security, repair or data-loss dialogs.
Do not terminate user-owned HWP processes. Require existing HWP processes to close.

Verified workstation context from the removed environment record:
DESKTOP-BRTN48S\MOLIT, interactive Session 1, Windows PowerShell 5.1 x64 STA,
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe,
Hancom Office 2020 HWP 11.0.0.9136, HWPFrame.HwpObject.
Confirm actual identity/session/host before COM work in an approved non-default
execution context. Sandbox results do not establish workstation registration.
Hidden windows in that interactive session are supported; services and concurrent
COM are not. Report when no live or visual test was performed.
For paragraph deletion select paragraph beginning through next paragraph beginning
(MoveSelNextParaBegin), then verify text. Do not use SetPos+SelectPara+Delete.
Keep native coordinates inside the adapter; no fixture coordinates in public IR.

Current shorthand: md2hwp source.md [[--output] <source.output.hwp>] creates source.ir.json and by
default source.output.hwp; backend takes source.ir.json [[--output] <source.output.hwp>]. Both
accept --template; Rust also accepts --worker and --dotnet. Option order is free.
Explicit option paths resolve from caller cwd, independently of image resources. Existing
option-based render modes remain. Defaults resolve beside the relevant EXE.
Shorthand resources resolve within the source/IR directory; explicit render modes
use cwd. Maintain source/template protection and successful-result replacement.

## Object sources (IR 0.4)

Object source is an optional nonempty array of closed {prefix, inlines} paragraphs.
Use only the approved labels in rules/ast2ir/ir-v0.4.json, case-insensitive for English.
Preserve spelling, punctuation, numbers and order. Optional period precedes an optional
ASCII digit suffix; digits are supported only for 주/주석/note. Require an immediate
colon followed by space and nonempty content. No space before colon. Attach consecutive
recognized paragraphs after figures/boxes/tables, allowing an intervening below-table
caption. Stop at the first nonmatching block; never absorb earlier unlabeled prose.
No ※ or footnotes in these notes; heading/figure number references are supported. Count every source paragraph and
prefix against cumulative limits. See docs/object-sources.md.
Require slot:<role>.source.prefix followed by ': ' and slot:<role>.source in each
box/figure/table source sample. Clone the complete template paragraph per IR paragraph
into the object's native caption or the table's single merged source cell. Preserve
sample paragraph formatting and each slot's first-character formatting; do not recompute
hanging indents from prefix lengths. Intermediate one-paragraph verification remains
adapter-private; final full-document save/reopen comparison covers all notes.

JPEG figures accept .jpg/.jpeg case-insensitively. Read bounded SOF0/SOF1/SOF2
8-bit frame headers after skipping length-delimited metadata; reject malformed,
truncated, zero-size and unsupported frame encodings explicitly. Pass the original
JPEG file directly to embedded InsertPicture, using the template width and pixel
aspect ratio. Hancom may re-encode or deduplicate embedded image data; do not
claim byte-for-byte preservation. Do not transcode JPEG into PNG or add format
fields to IR.

Hancom 2020 11.0.0.9136 native-caption capability verification: InsertCrossReference
succeeds inside picture, table and code-box CAPTION paragraphs; targets, cached
numbers and fields survive save/reopen. InsertFootnote returns false in all three
caption kinds, while the same-session body positive control succeeds. This is a
native insertion restriction on that version, not proof about every Hancom version
or forced HWPML structures. Keep caption footnotes forbidden, including heading
slots copied into CAPTION. Table sources live in merged cells: their footnotes
remain unsupported by the current object-note contract, not this caption test.
Allow heading/figure number references in figure captions/alt, table captions and
all object-note paragraphs, preserving surrounding formats and native AUTONUMs.
Figure alt remains descriptive metadata: validate its targets but do not expect
its reference markers in rendered paragraphs. Enumerate all object-note runs,
not only the first note; avoid double-enumerating intermediate source copies.
