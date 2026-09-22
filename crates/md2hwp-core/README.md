# md2hwp-core

The canonical Rust core loads the versioned product-owned AST-to-IR ruleset,
decodes and version-checks Pandoc JSON, normalizes supported Pandoc nodes, owns
the closed IR types and I/O, and enforces IR-local semantic and resource
validation.

The core does not inspect HWP/HWPX templates and does not lower IR into COM or
`rhwp` operations. Pandoc types do not leave normalization; backends receive
validated IR plus a template profile and runtime template path.

The crate owns IR 0.2 types and accepts historical 0.1 files against their
original closed schema. Optional verbatim-block sources require 0.2. Its I/O
boundary. `read_ir` rejects invalid UTF-8, duplicate JSON members, unsupported
versions, schema violations, semantic violations, and configured resource
limits. `write_ir` accepts only a `ValidatedDocument`, so invalid IR cannot be
persisted through the public writer.

The crate loads the exact built-in AST-to-IR 0.2 rules, accepts only the pinned
Pandoc JSON API version, applies closed normalization handlers, rejects
unsupported constructors with their AST paths, and validates the resulting IR.
Standalone images become figures; adjacent `출처: …` paragraphs attach to
figures and boxes. The core never reparses Markdown or resolves resource files.
