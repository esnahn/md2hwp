# AST-to-IR normalization rules v0.1

Status: historical ruleset. New normalization uses the
[0.2 rules and source attachment contract](ir-v0.2.md).

The authoritative data instance is
[`rules/ast2ir/ir-v0.1.json`](../../rules/ast2ir/ir-v0.1.json). Its closed
structural schema is
[`schemas/ast2ir-rules-v0.1.schema.json`](../../schemas/ast2ir-rules-v0.1.schema.json).

## Purpose and ownership

The ruleset makes the adopted Pandoc AST subset and simple normalization
policies inspectable without turning normalization into a scripting language.
It is a versioned product asset consumed by `md2hwp-core`, not an ordinary user
input and not a template profile.

The data answers which declared handler applies to each adopted constructor and
contains simple parameters such as accepted heading levels and ordered-list
variants. Constructors absent from the ruleset are rejected with their Pandoc
constructor and AST path.

The core owns the closed handler implementations and all recursive algorithms,
including AST traversal, list-context analysis, exact diagnostic paths, Unicode
preservation, typed IR construction, and semantic/resource validation. No rule
field may contain executable code, an expression, a dynamic path, or a fallback
handler.

## Required transformations

The v0.1 ruleset declares, among other mappings:

- Pandoc `Str` to IR `text`;
- Pandoc `Space` to IR `space`;
- Pandoc `SoftBreak` to IR `space`;
- Pandoc `LineBreak` to IR `line_break`;
- Pandoc `Para` to an IR `paragraph`;
- plain `CodeBlock` to `verbatim_block`;
- `Plain` only through the list-item paragraph handler;
- supported recursive `Strong`, `Emph`, and `Link` handlers.

Consequently `soft_break` is not an IR v0.1 inline variant. A serialized IR
file containing it is invalid. This intentionally loses whether prose
whitespace originated as a source newline or a literal space; md2hwp does not
round-trip source notation. Explicit source line breaks remain `line_break`, and
line-preserving verbatim blocks retain their `lines` representation.

## Loading and validation

Before decoding Pandoc blocks or inlines, the core must:

1. load exactly one explicitly selected built-in ruleset;
2. check `schema`, exact `rules_version`, and target IR envelope;
3. validate the closed structure and then decode it into closed typed rules;
4. reject unknown fields, constructors, handlers, and values;
5. verify rule-local invariants not expressible structurally.

The supported Pandoc API version and CLI invocation are checked separately by
the application before the Pandoc-input boundary. A runtime file with a
matching filename must never silently override the product ruleset.

## Versioning

Changing a mapping or parameter that can change normalized output requires a
new ruleset version and updated fixtures. A change that also changes the set or
shape of serialized IR nodes requires a new IR version after the first public
release.

JSON is the v0.1 serialization choice so the repository's existing Draft
2020-12 validation smoke can validate it without another declared runtime. The
architecture depends on a versioned data ruleset, not specifically on JSON.
