# Shared Rust crates

The shared library begins with `md2hwp-core`. It owns product-defined
AST-to-IR ruleset loading, Pandoc JSON decoding, normalization, the closed IR
types and I/O, and IR-local validation.

The user-facing Rust application lives under `apps/md2hwp`, not here. A
backend-specific Rust package may live inside its backend directory; Cargo
workspace members do not have to be children of `crates/`.

`md2hwp-core` is now a buildable Cargo package boundary; production conversion
logic has not been added yet. Each package uses Cargo's conventional `src/`
tree, and there is no top-level repository source tree.
