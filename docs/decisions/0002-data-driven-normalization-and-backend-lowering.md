# ADR 0002: Data-driven normalization and backend-owned lowering

- Status: accepted
- Date: 2026-08-26

## Context

The initial layout treated `md2hwp-core` as the owner of a shared
backend-neutral lowering stage and placed the user CLI beside it as another
crate. Design review exposed three distinct concerns:

- Pandoc JSON is a parser contract whose transport may be stdout, stdin, or a
  file;
- project IR should show canonical meaning, including `SoftBreak` already
  normalized to `Space`;
- Hancom Automation and a possible future `rhwp` implementation have different
  template models and native lowering methods.

A separate public Render Job representation would duplicate much of IR and was
not justified. At the same time, keeping normalization policy entirely hidden
in Rust code would make the adopted AST subset harder to inspect and audit.

## Decision

`md2hwp-core` owns the canonical Pandoc-JSON-to-IR boundary: ruleset loading,
closed handler implementations, normalization, IR types/I/O, and IR-local
validation. Product normalization policy is a versioned declarative ruleset in
`rules/ast2ir`, validated by a closed schema. It contains no executable code and
is not implicitly replaceable at runtime.

Pandoc invocation is a small module of the user-facing orchestrator under
`apps/md2hwp`; a separate reader package is not justified by launching one CLI.
The application may pass captured Pandoc JSON directly to the core without
persisting it.

Validated project IR is the common backend input and the only public persistent
intermediate document format. No public Render Job format is introduced. A
separate backend worker may use a small internal versioned invocation envelope,
but that is process transport rather than another document representation.

Every backend consumes validated IR, a template profile, and runtime
template/output paths. It owns actual-template inspection and binding,
backend-specific lowering, mutation, cleanup, and verification. Backends share
IR semantics, profile vocabulary, fixtures, and semantic postconditions rather
than native lowering operations.

Pandoc `SoftBreak` selects the `space` handler in the v0.1 AST-to-IR ruleset.
Serialized IR v0.1 therefore contains `space`, not `soft_break`; a schema smoke
example covers rejection of the latter.

## Consequences

The same source and ruleset yield inspectable canonical IR before any template
is opened. A template profile declares intended meanings, while each backend
must validate and bind those declarations against the actual HWP/HWPX file.

Backend implementations may duplicate traversal or formatting logic where
their native models differ. Semantic drift is controlled through common
conformance fixtures and output postconditions rather than by forcing a leaky
lowest-common-denominator operation format.

Ruleset changes that alter normalized output require versioning and fixture
updates. Complex recursive algorithms remain in typed core code; the data file
only selects closed handlers and simple parameters.
