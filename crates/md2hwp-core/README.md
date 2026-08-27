# md2hwp-core

Reserved for the canonical Rust core. It will load the versioned product-owned
AST-to-IR ruleset, decode and version-check Pandoc JSON, normalize supported
Pandoc nodes, own the closed IR types and I/O, and enforce IR-local semantic and
resource validation.

The core does not inspect HWP/HWPX templates and does not lower IR into COM or
`rhwp` operations. Pandoc types do not leave normalization; backends receive
validated IR plus a template profile and runtime template path.

This directory is a Cargo library package in the root workspace. Its current
`src/lib.rs` establishes the package boundary only; production IR types,
validation, normalization, and lowering are not implemented yet.
