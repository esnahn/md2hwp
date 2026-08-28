# ADR 0005: Pandoc 3.10.1 input contract

Status: accepted

## Context

The first CommonMark-to-IR slice needs a deterministic external parser. The
repository previously left the Pandoc release, API version, and reader options
open, so application code could not safely invoke or interpret Pandoc.

## Decision

- Adopt the official Pandoc 3.10.1 Windows x86_64 zip release. Its URL and
  SHA-256 are pinned in `dependencies/lock.json`.
- Install it explicitly under the ignored
  `.local/dependencies/pandoc/3.10.1/` tree. Dependency setup is never a build
  or test side effect.
- Require the executable's first `--version` line to be exactly
  `pandoc 3.10.1` before conversion.
- Invoke CommonMark as `--from=commonmark --to=json`. Pandoc 3.10.1 does not
  support the `auto_identifiers` extension for this reader and produces empty
  heading attributes by default.
- Invoke reStructuredText as `--from=rst-auto_identifiers --to=json` when that
  source path is implemented.
- Accept only `pandoc-api-version` `[1, 23, 1, 2]`, as observed from the pinned
  binary and recorded in compatibility fixtures.
- Pass source bytes through stdin and consume JSON from stdout. A nonzero exit
  status or nonempty unsupported AST constructor is a conversion failure, and
  no IR output is written.
- Neither the application nor `md2hwp-core` performs Unicode normalization.
  The pinned Pandoc CommonMark reader was nevertheless observed composing the
  literal sequence `U+1100 U+1161` to `U+AC00` before JSON reaches the core.
  This is recorded as an observed parser-boundary behavior, not an md2hwp
  normalization policy; see `docs/development/pandoc-unicode-normalization.md`.

The official release asset and checksum are published on the
[Pandoc 3.10.1 release](https://github.com/jgm/pandoc/releases/tag/3.10.1).
Pandoc documents both its JSON output format and reader extensions in the
[Pandoc User's Guide](https://pandoc.org/MANUAL.html).

## Consequences

The application can reproduce a single parser contract without relying on a
floating `PATH`. Upgrading Pandoc, changing a reader extension, or accepting a
different API version requires an explicit lock, fixture, and decision update.
