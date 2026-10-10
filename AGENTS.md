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
Standalone PDF CLI: apps/hwp2pdf/Hwp2Pdf.csproj. Publish its referenced backend
assembly inside hwp2pdf.exe; no template/Pandoc or separate worker is needed.
Use PrintToPDFEx through Action.Execute with GraphicQuality=100, Device=5,
full-document Range=6 and 100% scaling. Publish only after PDF completion,
source hash and normal COM cleanup checks; preserve old output on failure.
PDF envelope checks do not establish visual correctness or original-image preservation.
Template: templates/template.hwp. Build outputs: target/, bin/, obj/ (ignored).
Use target/debug and target/release as the primary paths for current development
builds and execution. target/dist is for versioned deployment archives; do not
redirect ordinary builds there merely to preserve an older release.
The JSON fixtures still tracked are included directly by Rust test compilation.
Builds include a deployment README.md with usable versioned documentation links.
Create release ZIPs with tools/development/build.ps1 -Configuration Release -Package;
include md2hwp.exe, md2hwp-backend.exe, hwp2pdf.exe, the tracked default template, README.md and the manuscript guides.
Maintain docs/manuscript/AGENTS.md and its concise human counterpart
docs/manuscript/README-MANUSCRIPT.md together; ship both at the package root.
Never ship the repository-root development AGENTS.md. Always replace
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
only in generated content. Figures embed PNG/JPG/JPEG/EMF images; caption and source belong to the picture's native caption.
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
IR cross_reference {kind: heading_number|figure_number|table_number, target: id}.
Require single-paragraph ref.figure.number and ref.table.number prototypes and six
begin/end:ref.heading1.number through ref.heading6.number one-paragraph blocks,
even when unused. Object number slots remain native Crossrefs. Heading references
are fixed inline text counted from manuscript heading order, independent of native
outline formatting, target paragraph identities and repeated heading block copies.
Remove the old ref.heading.number/slot:ref.heading.number declarations; do not retain
a native outline fallback. IR 0.4 is unchanged. Count headings without IDs too.
Heading1 follows md2hwp-heading1-start; deeper levels increment from 1 and reset on
any shallower heading. Missing parent levels retain 0. Resolve forward references
by target ID before rendering/width measurement, including alt/captions, all notes,
cells and footnotes, preserving source emphasis and verbatim body literals.
Heading reference blocks require their own num:headingN and may repeat own/ancestor
number tags; no deeper tags, controls, paragraph/page breaks or multiple paragraphs.
Prototype character formatting is ignored; source reference formatting is retained.
Generated heading references do not update when users edit headings in HWP.
Do not infer native number settings from ID prefixes or manuscript link labels.
No generated reference hyperlinks. Verify native field target/cache/format and
surrounding structures after save/reopen. Keep unreferenced template fields intact.
Use Action.Execute for sets returned by Action.CreateSet; passing those sets to
HAction.Execute fails COM interface conversion on this Hancom 2020 workstation.
Hancom reuses Crossref FieldId values across distinct fields. Require matching local
begin/end FieldId and unique field InstId; do not require global FieldId uniqueness.
Coalesce adjacent identical-format runs before HWPML import. Heading references
require no native outline settings; do not call GetHeadingString for references.
Page and other reference roles remain reserved and unsupported.
HWPX and RST input are deferred.

## Tables

Normalize simple Pandoc Table inputs to IR table {columns, header, rows, caption, source}.
Columns carry default/left/center/right alignment; header and row cells contain inline
arrays, including empty cells. Require one header row, one body, rectangular cells
and ColWidthDefault column widths only. Allow optional table IDs; reject other table attributes, merged cells, row-header
columns, multiple/intermediate headers or bodies, footer rows and nonparagraph or
multiple-paragraph cell blocks explicitly. Table number references require a native numbered table caption.
Cells allow rich text, footnotes and current heading/figure/table references; caption and
source reject footnotes but allow heading/figure/table number references. Include rows/cells and every inline/text in
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
Use num:heading1 through num:heading6 in heading blocks, numbered plain heading
samples and each.child scopes; resolve only own/ancestor levels from the same
manuscript counters as fixed heading references, including ID-less headings and
skipped parents. Bind repeated children using their own operation index, never
pre-fill descendants with the parent's counters. Title slots may have number/literal
affixes, one per plain paragraph; preserve the slot's first-character formatting,
rich manuscript title runs and independently formatted affixes. Keep existing native
list numbering optional and independent; do not silently disable it. Figure/table
captions and object-reference samples continue to allow only num:heading1.
Use adapter-private markers for template number tags so manuscript/metadata literals
are never interpreted recursively. Reject unresolved, orphan and deeper-level slots. Figure counters
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

Debug and Release builds compose the unpublished flat manuscript from validated IR
and template XML by default. Debug retains the original COM insertion path behind
--legacy-com for three-way comparison (Debug XML, Debug COM, Release XML).
Legacy option parsing/help is compiled only into Debug (Rust and backend).
Optimize the default Debug XML path; use Debug legacy COM as the comparison
baseline. Compile detailed RenderProfile metrics and call sites only into Debug,
enabled by MD2HWP_PROFILE=1. Release XML reads have no profiling wrappers/caller
metadata. Release measures total rendering time only via MD2HWP_TIMING=1;
this optional total timer also works in Debug and reports completion and elapsed
milliseconds without per-action metrics. Preserve document verification in both
configurations; instrumentation removal is not permission to weaken checks.
The live picture resource document supplies embedded data, native geometry and
object allocation order only, never expected manuscript/static content. Prime
body/master-page allocation with one native caption clone and discard that clone;
save/reopen the resource document to materialize image caches. Remap all imported
template format references because native block insertion can renumber definitions;
preserve static control identities and fields while remapping their formats.
Measure native list marker display only when continuation widths require it.
Keep final full-document import and save/reopen checks against the independently
composed expected XML. Do not weaken structural or stacking-order comparisons to
make the faster path pass. Performance and Debug/Release equivalence evidence is
recorded in docs/performance/2026-10-09-direct-xml.md; current three-way generation
evidence and profiles are in docs/performance/2026-10-09-generation-modes.md.

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
No ※ or footnotes in these notes; heading/figure/table number references are supported. Count every source paragraph and
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

EMF figures accept .emf case-insensitively without changing IR 0.4. Inspect the
little-endian EMR_HEADER and record/EOF boundaries in the backend before COM;
limit files to 64 MiB and one million records, reject invalid signatures/versions,
byte/record counts, offsets and nonpositive physical frames. Use rclFrame deltas
in 0.01 mm for aspect ratio, never treat rclBounds device units as pixels or fall
back silently when the frame is invalid. Pass the original file to embedded
InsertPicture; do not add a rasterization dependency or an IR format field.
The bounded container reader is not a full drawing-record semantic validator.
Hancom 2020 tests cover ordinary EMF and EMF+ Only/Dual. Compare native lossless
300/600dpi page images with direct Windows GDI+ rendering, retaining differences
and enlarged region strips; PDF export adds JPEG artifacts and is not the sole
display oracle. Do not claim cross-renderer pixel equality or independent engine
coverage merely from shared Windows rendering. See docs/emf-figures.md.

Hancom 2020 11.0.0.9136 native-caption capability verification: InsertCrossReference
succeeds inside picture, table and code-box CAPTION paragraphs; targets, cached
numbers and fields survive save/reopen. InsertFootnote returns false in all three
caption kinds, while the same-session body positive control succeeds. This is a
native insertion restriction on that version, not proof about every Hancom version
or forced HWPML structures. Keep caption footnotes forbidden, including heading
slots copied into CAPTION. Table sources live in merged cells: their footnotes
remain unsupported by the current object-note contract, not this caption test.
Allow heading/figure/table number references in figure captions/alt, table captions and
all object-note paragraphs, preserving surrounding formats and native AUTONUMs.
Figure alt remains descriptive metadata: validate its targets but do not expect
its reference markers in rendered paragraphs. Enumerate all object-note runs,
not only the first note; avoid double-enumerating intermediate source copies.

Table-number references follow xrefs_number semantics with existing CommonMark
attributes before pipe tables: a standalone {#id} immediately before the table,
not appended to the caption. Preserve Pandoc Table IDs in optional IR table.id;
IDs share the document-wide heading/figure namespace. Require ref.table.number
single-paragraph prototypes even when unused. Native table command kind is 0
(number display 1, hyperlink 0). Use target table Heading1Number for num:heading1,
never the reader chapter; keep native Table AUTONUM and existing restarts. Reject
referenced tables without a numbered native caption. Resolve generated table
shape identities before final import and verify field target, cache and formatting
after save/reopen, including captions, all notes, rich cells and footnote bodies.

Code, figure and table ranges may include 0–63 empty plain root paragraphs in total before and/or after the single native object anchor. Clone them on the corresponding sides of each generated object with their formatting, whitespace and page/column breaks. Reject text, controls and list/outline numbering. Attach spacing only after operation-indexed passes, before recording final layout paths; never space unrelated static template objects.
