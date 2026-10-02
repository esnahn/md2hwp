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

## Contracts

Keep Pandoc invocation in the app and Pandoc AST handling in the core. Backends
consume validated IR. Preserve Unicode. Reject unsupported constructors explicitly.
Use only the current IR 0.3 closed schema. AST2IR rules target that IR without
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
only in generated content. Figures embed PNGs; caption and source belong to the picture's native caption.
The begin:figure.caption/end:figure.caption sample contains one picture with a
two-paragraph native caption. Ignore sample image content/size; inherit caption options.
Native list.bullet/list.ordered sample paragraphs own bullet and per-level numbering
formats; manuscript start numbers and existing depth indentation remain authoritative.
Verbatim blocks may carry sources. Expand each raw IR line into a separate box
paragraph using the content sample's full formatting. Preserve spaces, tabs and
body blank lines. Optional slot:box.title precedes the content sample in the same
cell; only when this slot exists, interpret an exact first-line 제목: prefix as
the title, removing the prefix and one following space, then skipping immediately
following blank/whitespace-only lines. With no title slot, preserve all raw lines.
Keep this template-dependent presentation in the backend; do not add an IR title
field or parse Markdown syntax inside the remaining verbatim text.
Markdown input uses commonmark+yaml_metadata_block+footnotes. Reference-style
footnotes are enabled; inline_notes remains disabled. The 제목: and 출처: rules
are md2hwp conventions, not CommonMark or Pandoc extension syntax. Document these
separately in README.md.
IR inline footnote contains nonempty paragraph blocks. Allow notes in document
paragraphs, headings and list paragraphs, including formatted spans and link labels.
Footnote bodies allow multiple paragraphs and existing inline formatting/links/line
breaks. Reject nested footnotes, nonparagraph footnote blocks, and notes in figure
alt/caption or box/figure sources explicitly. Count footnote blocks, inlines and text
against the existing cumulative resource limits. Preserve native FOOTNOTE and
AUTONUM controls, then verify them after save/reopen.
Require a plain root paragraph {{md2hwp:footnote}} inside template definitions;
its full paragraph/character formatting owns generated note body paragraphs.
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
of new declarations or regeneration, even while the development IR remains 0.3.
Generated tables, HWPX and RST input are deferred.

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

Users may edit target/release/template.hwp directly. It can differ from the tracked
templates/template.hwp; never assume the tracked copy contains their latest work.
Never overwrite or modify the user-owned target/release/template.hwp for builds
or verification. build.ps1 copies a deployment template only when it is missing;
existing Debug and Release templates remain unchanged. Use a separate template
copy and explicit --template for experiments and live verification.
Update the tracked default only when the user authorizes adopting the edited copy.

Align release major/minor versions with the IR version (IR 0.3 corresponds to
v0.3.0). A Git tag alone does not update application/package version metadata or
README version labels; check their consistency when preparing a release.

Track heading1 from md2hwp-heading1-start, incrementing only for heading1.
Use num:heading1 inside heading blocks and native figure captions; preserve
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
