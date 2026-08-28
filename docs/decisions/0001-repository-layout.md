# ADR 0001: Product-boundary repository layout

- Status: superseded in part by ADR 0002, ADR 0003, and ADR 0004
- Date: 2026-08-26

## Context

The repository already contains normative IR files, third-party reference
material, generated HWP proofs, PowerShell investigation scripts, and a planned
Rust core. Without explicit boundaries, external binaries and exploratory COM
code could become accidental production dependencies.

## Decision

Use product boundaries for production code: reusable Rust library crates in
`crates/`, user applications in `apps/`, production Hancom integration in
`backends/`, template mappings in `profiles/`, and non-production automation in
categorized `tools/` folders. A Cargo workspace member may live outside
`crates/` when its product boundary belongs elsewhere.
Keep generated output, external source imports, workstation executables,
reference material, and distributable output in separate roots as specified in
[`docs/design/repository-layout.md`](../design/repository-layout.md).

The original layout placed `md2hwp-core` and `md2hwp-cli` together under
`crates/`. ADR 0002 retains the product-boundary principle but moves the
user-facing application to `apps/md2hwp`, introduces the ruleset boundary, and
makes lowering backend-specific.

ADR 0003 retains the product-boundary roots, selects C#/.NET for the Hancom
worker, and replaces the premature common distribution staging root with
language-native project and build-output conventions.

ADR 0004 consolidates external pins in `dependencies/lock.json`, keeps Pandoc
process invocation in the application and backend integrations under
`backends/`, and reserves `upstream/pandoc` and `upstream/rhwp` for pinned Git
submodules.

## Consequences

Production builds cannot depend implicitly on local reference files or
investigation scripts. External source and binaries require provenance.
Additional top-level language folders are unnecessary. Each component uses the
package, project, test, and generated-output conventions of its own ecosystem.

At the time of this decision, Rust/MSRV, Pandoc, the backend implementation
language, packaging format, Git hosting, Git LFS, and the project license were
not selected. Later choices are recorded by the superseding ADRs above.
