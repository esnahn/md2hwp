# ADR 0003: Language-native project layout and build output

- Status: accepted; external payload placement amended by ADR 0004
- Date: 2026-08-26

## Context

The repository has Rust application/core boundaries, a Windows Hancom
Automation worker boundary, and declarative rules, schemas, and template
profiles. A single repository-wide `src/bin/dist/share` hierarchy would mix
Cargo packages, .NET projects, source data, compiler output, and installation
layout. The pre-implementation layout also declared `dist/` as the sole release
staging root even though no package or installer format had been selected.

## Decision

Keep top-level product boundaries and apply conventions inside each boundary:

- Rust application and library packages use Cargo manifests and their standard
  `src/main.rs` or `src/lib.rs` layouts. Cargo-generated output stays in
  ignored `target/` directories.
- The Hancom Automation worker is implemented in C#/.NET. Each production or
  test project owns its `.csproj` and C# files, and tests use a separate test
  project. Generated `bin/` and `obj/` directories are ignored. The .NET SDK,
  target framework, solution/project names, and COM interop strategy require a
  later decision before scaffolding.
- Declarative rules, schemas, and profiles remain in role- and version-named
  data trees. They are not placed below a generic `src/` or `share/` directory.
- Remove the predeclared top-level `dist/` boundary. Once a release format is
  selected, its package or installer tooling defines and ignores generated
  staging/output paths. Tracked definitions continue to belong in `packaging/`.

There is no tracked generic `bin/` tree and no Unix-style `share/` installation
tree. Unrelated workstation tools remain outside the repository and are found
through the workstation's ordinary tool configuration. ADR 0004 places the
ignored Hancom security-module DLL beside its installer under `dependencies/`;
other downloaded payloads require an explicitly documented ignored destination.

## Consequences

Source locations express product ownership while each implementation remains
recognizable to its ecosystem's tools. Build output is found where Cargo and
.NET developers expect it, without making compiler directories part of the
architecture. Release layout remains open until the actual Windows/package
format can define it, and packaging may embed or copy data without relocating
the canonical data sources.
