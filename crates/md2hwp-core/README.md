# md2hwp-core

Reserved for the canonical Rust core. It will load the versioned product-owned
AST-to-IR ruleset, decode and version-check Pandoc JSON, normalize supported
Pandoc nodes, own the closed IR types and I/O, and enforce IR-local semantic and
resource validation.

The core does not inspect HWP/HWPX templates and does not lower IR into COM or
`rhwp` operations. Pandoc types do not leave normalization; backends receive
validated IR plus a template profile and runtime template path.

The crate now owns the closed IR v0.1 types and its first validated I/O
boundary. `read_ir` rejects invalid UTF-8, duplicate JSON members, unsupported
versions, schema violations, semantic violations, and configured resource
limits. `write_ir` accepts only a `ValidatedDocument`, so invalid IR cannot be
persisted through the public writer.
