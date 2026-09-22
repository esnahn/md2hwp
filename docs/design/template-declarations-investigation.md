# Explicit template declarations: investigation draft

Status: non-production lexical contract for ADR 0008. No HWP reader or renderer
consumes this grammar yet. Native field/bookmark comparison remains required.

## Three different scopes

| Declaration | Exact meaning |
| --- | --- |
| `{{md2hwp:content}}` | One dedicated root paragraph: replace that paragraph with generated blocks. |
| `{{md2hwp:body}}`, `{{md2hwp:heading.1}}` through `.6` | One dedicated sample paragraph: use its native paragraph style. No following-paragraph inference. |
| `{{md2hwp:begin:ROLE}}` / `{{md2hwp:end:ROLE}}` | Explicit ordered pair of dedicated root paragraphs enclosing a complete prototype. |

`ROLE` is `samples`, `block.box`, `figure`, `list.bullet`, or `list.ordered`.
Exactly one `samples` range owns all paragraph samples and prototypes. The
content insertion paragraph is outside that range. The template contains
exactly one `{{md2hwp:contract:experimental-1}}` paragraph inside `samples`.
Body is required; other roles may be omitted until requested by IR. Duplicate
role definitions are errors even if formatting happens to match.

All spelling is case-sensitive. No trimming, expressions, inferred aliases,
or fuzzy matching. A declaration occupies its whole paragraph; CR/LF inside
that paragraph is invalid. Text runs may split its spelling: a future reader
must reconstruct paragraph text while retaining native paragraph identity.
Recognize declarations only in the template before inserting any IR content.

## Authoring example

Each line below denotes a paragraph or an explicitly described native object,
not a plain-text HWP serialization:

```text
[unchanged report front matter]
{{md2hwp:content}}
[unchanged report back matter]
{{md2hwp:begin:samples}}
{{md2hwp:contract:experimental-1}}
{{md2hwp:body}}                 <- entire paragraph, in desired body style
{{md2hwp:heading.1}}            <- entire paragraph, in desired heading style
{{md2hwp:begin:block.box}}
[inline box/table anchored in a body paragraph]
    [inside its content cell] {{md2hwp:slot:box.content}}
    [existing source decoration, if required by the adopted box policy]
{{md2hwp:end:block.box}}
{{md2hwp:begin:figure}}
{{md2hwp:slot:figure.image}}    <- dedicated body paragraph, image insertion slot
[native caption AUTONUM] {{md2hwp:slot:figure.caption}}
{{md2hwp:slot:figure.source}}
{{md2hwp:end:figure}}
{{md2hwp:end:samples}}
```

The arrows/indentation are explanatory and must not appear in the declaration
paragraphs. Caption AUTONUM is a native control, not text; the caption slot
is the paragraph's entire textual content. A later reader must retain that
control and verify its location, number type, and formatting independently.

## Range and slot semantics

For prototype boundary paragraphs B and E, capture the half-open native range
`[begin(next paragraph after B), begin(E))`. Neither boundary paragraph belongs
to the cloned prototype. Boundaries must share the same root flow and section;
they cannot be inside a cell, caption, header, or footer. All included objects
must have their owning anchors inside the range. Do not infer extent from
visual proximity, style, page position, or a matching caption elsewhere.

Only `samples` may contain prototype ranges; prototype ranges cannot nest or
overlap. An end must match the open role. Empty ranges are invalid. Declare
every prototype once. Each slot below must occur exactly once within its owner:

| Owner | Required slots |
| --- | --- |
| `block.box` | `box.content` |
| `figure` | `figure.image`, `figure.caption`, `figure.source`, in that order |
| `list.bullet` | `list.item` |
| `list.ordered` | `list.item` |

The box slot identifies a content paragraph inside the explicitly enclosed
box, not the box's outer extent. List samples carry native bullet/numbering
properties; native nesting behavior must be tested separately. Figure image
placement is inline, source immediately follows caption, and width/aspect
policy still requires an explicit validated setting in the eventual contract.
Slots in a different owner's range, orphan slots, unknown reserved tags,
unmatched or crossed ends, and repeated slots are errors.

The token scanner below checks paragraph order and lexical ownership only.
It cannot prove cell ownership, root flow, section identity, style availability,
picture anchoring, native numbering, adjacency of native paragraphs, or clone
fidelity. A successful lexical check is never permission to render.

## Removal and preservation

Capture and validate all prototypes before mutating the output copy. Remove
the complete samples area including its boundaries using paragraph-beginning
ranges; a native successor paragraph must exist. Replace the dedicated content
paragraph through its successor beginning. Resolve fresh native positions after
each edit instead of reusing stale coordinates. Verify complete surrounding
text and structures before and after each deletion. Preserve the input hash.
Repeated rendering starts from the original template, never from prior output.

No boundary tags, sample paragraphs, or slot tokens remain in output. A tag-like
string supplied by IR is ordinary content and is not scanned a second time.

## Executable lexical experiment and next gates

`TemplateDeclarations.cs` in the C# investigation preview accepts ordered
paragraph strings and returns paragraph indices for explicit ranges and slots.
These are investigation indices, not COM coordinates or a public document IR.
It is deliberately disconnected from the render path. Run the fixture harness:

```powershell
pwsh -NoProfile -File .\tools\development\dotnet.ps1 run --project .\tools\investigation\hancom-automation\template-declarations-tests\TemplateDeclarations.Tests.csproj
```

Next: compare this explicit-range candidate against named native fields on a
separate HWP fixture. Run the canonical open-only probe in the verified context
first. Demonstrate exact range capture, save/reopen persistence, duplicate
rejection, native-slot ownership, content preservation, and page-image checks.
Only then adopt a native representation and a production declaration schema.

The existing box/caption investigators already use HWP `saveblock` and
`insertfile`; they do not prove arbitrary multi-paragraph range fidelity. The
[Hancom Automation response on GetTextFile/SetTextFile](https://forum.developer.hancom.com/t/gettextfile-hwpx-saveblock/3102/2)
also states these methods do not support HWPX. This experiment must not extend
its HWP claims to HWPX.
