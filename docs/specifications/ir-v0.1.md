# md2hwp Project IR v0.1

Status: normative pre-implementation serialization contract for IR version
`0.1`. The pre-release `SoftBreak` correction is recorded in ADR 0002.

The project IR is the stable boundary between source-language normalization and
document lowering. It represents document meaning, not Pandoc constructors,
template coordinates, Hancom Automation actions, or `rhwp` implementation
details.

The machine-readable structural contract is
[`schemas/ir-v0.1.schema.json`](../../schemas/ir-v0.1.schema.json). Rules marked as
semantic invariants below are checked by the typed decoder and semantic
validator because JSON Schema alone cannot express them reliably.

## 1. Document envelope

Every IR file is a UTF-8 JSON document with this envelope:

```json
{
  "schema": "md2hwp.ir",
  "ir_version": "0.1",
  "metadata": {},
  "blocks": []
}
```

- `schema` and `ir_version` are exact string constants. A reader MUST inspect
  them before decoding the rest of the document and MUST reject unknown
  versions.
- `metadata` MUST be an empty object in v0.1. CommonMark and reStructuredText do
  not share a sufficiently precise document-metadata contract. Non-empty
  Pandoc metadata therefore fails normalization instead of being discarded.
- `blocks` is ordered and may be empty.
- Unknown object members and unknown node types are errors.
- Duplicate member names in a JSON object are errors; a decoder MUST NOT use
  first-wins or last-wins behavior.
- JSON object member order is not meaningful.

IR version numbers and product milestone names are independent. Adding a table
or footnote node would require a new IR version even if the product calls that
work “v2”.

## 2. Type model

```text
Document {
  schema: "md2hwp.ir",
  ir_version: "0.1",
  metadata: {},
  blocks: Block[]
}

Block =
  Paragraph {
    type: "paragraph",
    inlines: Inline[1..]
  }
  | Heading {
    type: "heading",
    level: 1..6,
    inlines: Inline[1..]
  }
  | VerbatimBlock {
    type: "verbatim_block",
    lines: String[1..]
  }
  | List {
    type: "list",
    kind: "bullet" | "ordered",
    start: Integer?,
    tight: Boolean,
    items: ListItem[1..]
  }
  | Figure {
    type: "figure",
    image: ImageRef,
    caption: Inline[1..],
    source: null | Inline[1..]
  }

ListItem {
  blocks: Paragraph, (Paragraph | List)*
}

ImageRef {
  path: String,
  alt: Inline[],
  title: null | String
}

Inline =
  Text { type: "text", value: String }
  | Space { type: "space" }
  | LineBreak { type: "line_break" }
  | Strong { type: "strong", inlines: Inline[1..] }
  | Emph { type: "emph", inlines: Inline[1..] }
  | Link {
      type: "link",
      target: String,
      title: null | String,
      inlines: Inline[1..]
    }
```

## 3. Inline semantics

### 3.1 Text and breaks

- `text.value` is non-empty and contains neither U+0020 SPACE nor a C0/DEL
  control character (U+0000 through U+001F and U+007F). U+0020 is represented
  by a `space` node.
- A normalizer MUST NOT apply NFC, NFD, compatibility normalization, case
  folding, or replacement-character repair. Unicode scalar values are
  preserved exactly.
- Adjacent `text` nodes at the same nesting level SHOULD be merged by writers.
- Pandoc `SoftBreak` is normalized to `space` before IR construction. IR does
  not preserve whether prose whitespace originated as a source newline or a
  literal space, and `soft_break` is not a serialized IR node.
- `line_break` lowers to a line break inside the same paragraph. It is neither
  a paragraph break nor a page break.

### 3.2 Strong and emphasis

`strong` and `emph` remain nested semantic nodes in IR. Each backend's lowering
walks the tree with an active mark set and coalesces adjacent runs with the same
set:

```text
Strong(Text("굵게"), Space, Emph(Text("둘"), Space, Text("다")))

=>

Run("굵게 ", marks={strong})
Run("둘 다", marks={strong, emph})
```

The backend applies these marks as patches over the resolved base character
shape. It MUST NOT replace unrelated font, size, color, spacing, or language
properties. Repeated nesting of the same mark is semantically idempotent.

### 3.3 Links

IR preserves the link label, target, and optional title. Link attributes that
cannot be represented by this contract are rejected during normalization.
Links may contain `strong` and `emph`; nested links are a semantic validation
error.

An absent title is serialized as `null`; an empty title string is not
canonical and is rejected. A link target is non-empty, so a CommonMark empty
destination such as `[label]()` is outside v0.1 rather than guessed. Targets
and titles contain no C0 control or DEL.

The AURI v0.1 template profile deliberately renders only the recursively
lowered label text and formatting. Other template profiles or backends may
create an active hyperlink. The target is not discarded from IR.

## 4. Block semantics

### 4.1 Paragraph and heading

- A paragraph carries prose content, not a presentation selector. Its body
  presentation is a template-profile and backend lowering policy.
- A heading level determines its presentation; that policy is not duplicated
  as a style field in the heading node.
- Empty paragraphs and headings are not representable in v0.1.

### 4.2 Verbatim block

A verbatim block is a general line-preserving block, not a
programming-language or law-specific type.

- Every `lines` element contains no CR or LF.
- TAB (U+0009) is allowed in a verbatim-block line. Other C0 controls and DEL are
  rejected.
- Joining elements with LF reconstructs the logical block text. Empty elements
  preserve blank lines, including a final blank line.
- An empty block is represented as `lines: [""]`.
- A plain CommonMark fenced/indented code block or reStructuredText literal
  block maps to `verbatim_block`.
- A code language, identifier, class, or key/value attribute has no v0.1 field.
  A non-empty Pandoc `Attr` or info string therefore fails unless a future,
  separately specified normalizer rule consumes it without loss.
- The AURI basic profile renders a verbatim block using its `박스내용`
  presentation. That concrete style name and the profile's internal binding are
  not stored in IR.

### 4.3 Lists

- `kind: "bullet"` MUST omit `start`.
- `kind: "ordered"` MUST include `start` with a value of at least 1.
- `tight` preserves the source list's tight/loose rendering distinction.
- Ordered-list marker style and delimiter are template-profile policies in
  v0.1. A normalizer accepts `DefaultStyle` or `Decimal` and accepts
  `DefaultDelim`, `Period`, or `OneParen`; these values intentionally normalize
  to the profile's native numbering. Roman, alphabetic, example, and
  two-parenthesis styles are rejected rather than discarded.
- A list item contains one or more paragraphs and/or nested lists. Headings,
  figures, and verbatim blocks inside list items are not representable in v0.1.
- The first block of every item MUST be a paragraph so the native list marker
  has a semantic anchor. An item containing only a nested list is invalid.
- A tight list item contains exactly one direct paragraph, followed by zero or
  more nested lists. A loose list item may contain continuation paragraphs and
  nested lists. During Pandoc normalization, list-level tightness is derived
  from `Plain` versus `Para`; `Plain` is converted to an IR paragraph only in a
  list item, and a mixed/ambiguous structure is rejected instead of guessed.
- List paragraphs have the same style-free shape as top-level paragraphs. A
  backend applies the selected profile's list and body presentation policies.
- A backend renders native bullet or paragraph-numbering structures. Literal
  marker text such as `- ` or `1. ` is not a valid fallback.

### 4.4 Figures and assets

- `image.path` is a non-empty relative local path resolved from the IR file's
  directory. Writers use `/` as the canonical separator. A leading `/`, drive
  prefix, URI scheme, UNC form, backslash, C0 control, or DEL is invalid.
- `image.title` is `null` when absent; otherwise it is a non-empty string with
  no C0 control or DEL. An empty title is rejected rather than treated as a
  second spelling of absence.
- `.` and `..` segments are allowed in serialized form so an IR file may refer
  to a sibling project asset, as the example does. Job preflight resolves the
  normalized path, including symlinks or Windows reparse points, and requires
  the final path to remain inside the configured project/input root.
- Before output is written, preflight resolves the path and verifies that it is
  inside the configured project/input root, readable, and supported. Remote
  fetches are outside v0.1.
- `caption` is required and non-empty.
- `caption` contains the human-authored caption text only. A template-owned
  label or sequence number such as `그림 1` is not literal IR text.
- `source: null` means that no source metadata was supplied. A template profile
  may still instantiate its required empty source placeholder.
- `source` content, when present, uses normal inline semantics and fills the
  semantic source slot. Template-owned prefixes and labels are not duplicated
  in IR.
- The generic Markdown/reStructuredText syntax that creates a `figure` and its
  source line is not yet defined. A normalizer MUST NOT infer it from image
  titles or surrounding prose. The node is available for trusted direct IR and
  for a future explicitly documented input mapping.
- Figure width, inline placement, caption numbering, and source-line layout are
  template-profile policies, not IR fields. The initial AURI profile preserves
  aspect ratio and limits width to 142 mm.

## 5. Adopted source-language subset

IR v0.1 represents the adopted subset that CommonMark and reStructuredText can
normalize to the same meaning. It is not a claim that every feature of either
language is supported.

| Meaning / Pandoc form | v0.1 policy |
| --- | --- |
| Paragraph (`Para`) | Adopt as `paragraph` |
| Heading (`Header`) | Adopt levels 1 through 6 |
| Text and space | Adopt distinctly |
| Soft break | Normalize to IR `space` |
| Hard/explicit line break | Adopt as IR `line_break` |
| `Strong`, `Emph` | Adopt and preserve nesting |
| `Link` | Preserve label, target, and title; AURI renders label only |
| Bullet and ordered lists | Adopt, including tightness and nested lists |
| `Plain` | Accept only as a list-item paragraph during normalization |
| Plain code/literal block | Adopt as `verbatim_block` |
| Figure | Representable; source syntax/mapping still requires a decision |
| Inline code | Reject in v0.1 |
| Block quote | Reject in v0.1 |
| Inline image not normalized as a figure | Reject in v0.1 |
| Raw block / raw inline | Reject |
| Horizontal/thematic rule | Reject |
| Table | Deferred; intended for a later IR version |
| Footnote/endnote | Deferred; intended for a later IR version |
| Page break | Not representable in v0.1 |
| Any unlisted constructor | Reject |

Normalization MUST use an explicit constructor allowlist. Rejection of a
Pandoc node reports at least:

```json
{
  "error_version": "0.1",
  "code": "unsupported_pandoc_node",
  "message": "BlockQuote is not supported by IR 0.1",
  "path": "/blocks/3",
  "constructor": "BlockQuote",
  "reader": "commonmark"
}
```

`path` is an RFC 6901 JSON Pointer into the Pandoc JSON. Diagnostic objects are
not embedded in an IR document, and a failed normalization MUST leave no
partial IR output.

Stable v0.1 diagnostic codes include `unsupported_pandoc_node`,
`unsupported_pandoc_api_version`, `unsupported_ir_version`,
`invalid_ir_schema`, and `invalid_ir_semantics`. Tests compare the stable code,
path, constructor, expected value, and actual value where present; prose
messages may improve without becoming an API.

Pandoc `Attr` data is accepted only where this specification gives it a
lossless mapping. In v0.1 that mapping is not defined. `Header`, `Link`,
`CodeBlock`, `Image`, `Figure`, and any other adopted constructor carrying an
`Attr` therefore require the exact empty value `["", [], []]` (or reader
options that produce it). A non-empty identifier, class list, or key/value list
is a normalization error. Reader-generated automatic heading identifiers must
be disabled or rejected; they are never silently removed.

## 6. Validation boundary

IR validity and template/backend renderability are separate. Both checks
complete before template mutation.

IR decoding and self-contained semantic validation perform:

1. Parse JSON as UTF-8.
2. Read and check `schema` and `ir_version`.
3. Validate the closed JSON structure against the v0.1 schema.
4. Decode into closed typed variants.
5. Enforce IR-local semantic invariants, including:
   - no nested links;
   - canonical path syntax and list shape;
   - configured parser nesting and document-size limits.

Renderability preflight then performs:

1. Resolve asset paths, filesystem containment, existence, readability, and
   supported media types.
2. Resolve the presentation requirements implied by each semantic node in the
   selected template profile.
3. Compute the IR-dependent backend/template capability requirements.
4. Reject a missing capability, ambiguous target, or format mismatch before an
   output copy is mutated.

An IR can be intrinsically valid but not renderable with a particular template,
asset root, or backend. That outcome is a preflight failure, not a different IR
interpretation.

No unknown node or unsupported capability may silently fall back to plain text
or a blank paragraph.

## 7. Backend boundary

Each backend lowers the same validated IR into its native document model and is
tested against the same semantic postconditions. The following must not appear
in an IR file:

- COM actions, cursor positions, selections, paragraph numbers, or HWP units;
- concrete HWP style ids or template paragraph coordinates;
- HWP/HWPX package node paths or relationship ids;
- `rhwp` object ids or serializer state.

A Hancom Automation backend may create formatted runs through character-shape
changes or verified rich fragments. A future `rhwp` backend may create the same
runs in its own document model. Their lowering implementations need not share
native operations, but both must satisfy the same semantic postconditions and
preserve the source template format (HWP to HWP, HWPX to HWPX).

## 8. Versioning

- Readers explicitly enumerate supported versions; an unknown minor version is
  rejected during the v0.x period.
- After the first public release, adding, removing, or changing a serialized
  field or union variant requires a new IR version. Before any converter or
  persisted consumer existed, the unreleased v0.1 contract removed
  `soft_break`; ADR 0002 records that one-time correction.
- Editorial clarification that does not change accepted JSON may update this
  document without changing `ir_version`.
- There is no `unknown`, `extension`, or untyped escape hatch in v0.1.

Open decisions outside this contract remain the exact Pandoc API/reader option
set, the source syntax for figures and source metadata, and concrete
template-profile mappings.
