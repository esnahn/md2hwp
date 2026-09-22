# Project IR v0.2

Status: implemented. This is a narrow extension of the [IR 0.1 contract](ir-v0.1.md).
The normative closed schema is [ir-v0.2.schema.json](../../schemas/ir-v0.2.schema.json).

The envelope uses `ir_version: "0.2"`. All 0.1 nodes, Unicode invariants,
resource limits, image-path semantics, and rejected constructs remain unchanged,
except that `VerbatimBlock` adds an optional nullable source:

```text
VerbatimBlock { type: "verbatim_block", lines: String[1..], source?: null | Inline[1..] }
```

Missing and null mean absent. The Rust serializer omits absent box sources.
Nonempty sources use the existing recursively validated inline vocabulary,
including Strong, Emph, and Link. Source text is metadata belonging to the box,
not another document paragraph. It does not include the template's source label.
Figure already has its required nullable source member; that shape is unchanged.

Readers accept 0.1 against the original 0.1 schema and preserve its envelope.
They reject box source fields in 0.1, including explicit null. Typed validation
also rejects a non-null box source on a 0.1 document. New normalization emits
0.2; callers must not change the version number to smuggle fields into 0.1.
Versions other than 0.1 and 0.2 are rejected before schema validation.

Tables, generated footnotes and page breaks remain outside this IR version.

## AST-to-IR ruleset 0.2

[rules/ast2ir/ir-v0.2.json](../../rules/ast2ir/ir-v0.2.json) and its
[closed schema](../../schemas/ast2ir-rules-v0.2.schema.json) replace the built-in
0.1 rules for new normalization. The old data assets remain for historical
validation. Existing mappings are unchanged except:

- Top-level `Para` containing exactly one attribute-free `Image` becomes Figure.
  Its alt inlines serve as caption and alt; the image target and optional title
  are preserved. Inline, multiple and list-nested images remain unsupported.
- A single immediately following `Para` with plain `Str("출처:")`, `Space`,
  and nonempty supported inlines attaches to a Figure or VerbatimBlock.
  It is removed from the top-level block stream. Prefix and separator nodes
  still undergo closed AST validation. Errors retain original Pandoc paths.
- A source paragraph outside that position remains prose. An empty reserved
  source paragraph immediately after an eligible object is rejected.

The input grammar and scope are adopted in [ADR 0009](../decisions/0009-commonmark-object-sources.md).
The application rebases relative image paths from the source file's directory
to the IR output directory, then validates again before serializing. The core
does not access files or reinterpret Markdown.
