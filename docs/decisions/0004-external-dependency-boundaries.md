# ADR 0004: Consolidated dependency lock and upstream submodules

- Status: accepted
- Date: 2026-08-26

## Context

Pandoc, rhwp, and Hancom Automation arrive in different forms, but separate
metadata directories for each made the repository harder to scan without
adding executable value. A dedicated `readers/pandoc` boundary was also larger
than its job: select fixed CLI arguments, launch `pandoc`, capture JSON, and
propagate diagnostics.

Developers still need the Pandoc and rhwp source available for API/format
inspection. A generic imported-source directory does not express that these are
specific upstream repositories, while ordinary copied checkouts do not carry a
mechanical revision pin.

## Decision

Use one closed `dependencies/lock.json` for every adopted external dependency.
Each entry records its consumer and one or more exact system, content, release,
or Git-submodule pins. Do not create a metadata directory or lock per
dependency.

Keep Pandoc process invocation inside `apps/md2hwp` until it grows enough or is
reused enough to justify another package. Pandoc is a Haskell CLI; it is not
linked into the Rust application. `md2hwp-core` begins at Pandoc JSON decoding
and owns normalization semantics.

When the parent repository is initialized and compatible revisions are
selected, add these Git submodules:

```text
upstream/pandoc  -> https://github.com/jgm/pandoc
upstream/rhwp    -> https://github.com/edwardkim/rhwp
```

The parent gitlink mechanically pins each source checkout. The corresponding
`git-submodule` entry in `dependencies/lock.json` repeats the auditable
repository, revision, path, and consumer. A smoke check must reject disagreement
between the gitlink and lock once the submodules exist.

`upstream/pandoc` is source-inspection material; normal conversion uses a pinned
Pandoc release executable. `upstream/rhwp` is source-inspection material and may
later be consumed by `backends/rhwp` as a Rust path dependency or built CLI,
after an explicit compatibility decision.

The specialized Hancom security-module installer stays directly under
`dependencies/` and reads the Hancom entry from the consolidated lock.
Its hash-verified DLL remains outside version control at the exactly ignored
`dependencies/FilePathCheckerModuleExample.dll`. Any other downloaded payload
requires its own explicitly documented ignored destination or a documented
system installation location.

## Consequences

The repository removes `readers/`, `third_party/`, and the three
`dependencies/<name>` metadata directories. Source retrieval uses ordinary
`git submodule update --init --recursive`, and the only exceptional setup file
is the Hancom security installer.

The current workspace has no `.git` metadata. Actual submodules, `.gitmodules`,
Pandoc/rhwp lock entries, and any installer tied to those pins are therefore
created only after repository initialization and compatibility selection; no
floating branch or current-head revision is silently adopted.
