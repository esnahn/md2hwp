# Repository layout

Status: accepted repository contract. The ADRs record the decisions; this
document is the authoritative current map.

The repository uses product boundaries only where code or data has a distinct
owner. A one-command external process does not receive its own package, and
external metadata is consolidated rather than split by dependency.

## File-level map

Legend:

- `[tracked]`: present file intended for version control when the repository is
  initialized;
- `[planned]`: create only after the named decision is resolved;
- `[submodule]`: pinned upstream gitlink, materialized by Git;
- `[local]`: ignored local input, cache, downloaded payload, or output.

```text
md2hwp/
├── [tracked] AGENTS.md
├── [tracked] README.md
├── [tracked] .editorconfig
├── [tracked] .gitattributes
├── [tracked] .gitignore
├── [planned] .gitmodules               With the first upstream gitlink
├── [tracked] Cargo.toml                Rust workspace and shared metadata
├── [tracked] Cargo.lock                Application dependency graph
├── [tracked] rust-toolchain.toml       Exact Rust toolchain and components
├── [tracked] global.json               Exact repository-local .NET SDK
│
├── apps/
│   ├── [tracked] README.md
│   └── md2hwp/
│       ├── [tracked] README.md
│       ├── [tracked] Cargo.toml
│       └── src/
│           └── [tracked] main.rs        CommonMark-to-IR CLI and Pandoc launch
│
├── crates/
│   ├── [tracked] README.md
│   └── md2hwp-core/
│       ├── [tracked] README.md
│       ├── [tracked] Cargo.toml
│       └── src/
│           ├── [tracked] lib.rs         Public core boundary
│           ├── [tracked] ir.rs          Closed IR types
│           ├── [tracked] ir_io.rs       Strict validated IR I/O
│           ├── [tracked] ast2ir_rules.rs
│           ├── [tracked] pandoc_input.rs
│           ├── [tracked] normalize.rs
│           └── [tracked] validate.rs
│
├── backends/
│   ├── [tracked] README.md
│   ├── hancom-automation/
│   │   ├── [tracked] README.md
│   │   └── [planned] .NET solution, product project, and test project
│   └── rhwp/
│       ├── [tracked] README.md
│       └── [planned] md2hwp integration package/files
│
├── upstream/                            Created by Git, not empty scaffolding
│   ├── [submodule] pandoc               jgm/pandoc at an adopted revision
│   └── [submodule] rhwp                 edwardkim/rhwp at an adopted revision
│
├── dependencies/
│   ├── [tracked] README.md
│   ├── [tracked] lock.json              All adopted external pins
│   ├── [tracked] install-hancom-security-module.ps1
│   ├── [tracked] install-pandoc.ps1
│   ├── [tracked] install-dotnet-sdk.ps1
│   └── [local] FilePathCheckerModuleExample.dll  Ignored pinned payload
├── .local/dependencies/pandoc/          Ignored locked portable parser
├── .local/dependencies/dotnet/          Ignored locked portable SDK
├── .local/state/                        Ignored tool state and caches
│
├── rules/
│   ├── [tracked] README.md
│   └── ast2ir/ir-v0.1.json
├── schemas/
│   ├── [tracked] README.md
│   ├── [tracked] ast2ir-rules-v0.1.schema.json
│   ├── [tracked] dependencies-lock-v0.1.schema.json
│   ├── [tracked] ir-v0.1.schema.json
│   └── [tracked] template-profile-v0.1.schema.json
├── profiles/
│   ├── [tracked] README.md
│   └── templates/auri-basic/
│       ├── [tracked] README.md
│       └── [tracked] investigation-v0.1.json
│
├── examples/
│   ├── [tracked] commonmark-v0.1.md
│   ├── [tracked] commonmark-v0.1.expected.ir.json
│   ├── [tracked] ir-v0.1.json
│   ├── [tracked] ir-v0.1-rejected-non-nfc.json
│   ├── [tracked] ir-v0.1-rejected-page-break.json
│   └── [tracked] ir-v0.1-rejected-soft-break.json
├── tests/
│   ├── [tracked] README.md
│   ├── [tracked] fixtures/pandoc-json/commonmark-v0.1.json
│   ├── [planned] expected/
│   ├── [planned] integration/hancom-automation/
│   ├── [planned] integration/rhwp/
│   └── [planned] visual/
├── tools/
│   ├── [tracked] README.md
│   ├── development/{show-environment.ps1,dotnet.ps1}
│   ├── investigation/auri/{README.md,build-format-examples.ps1}
│   ├── investigation/hancom-automation/ir-preview/
│   │   └── [tracked] investigation C# project and launcher
│   └── smoke/{test-contracts.ps1,test-commonmark-to-ir.ps1,
│              test-pandoc-json-to-ir.ps1,test-hancom-ir-preview-plan.ps1}
├── docs/
│   ├── [tracked] README.md
│   ├── design/repository-layout.md
│   ├── development/environment.md
│   ├── specifications/{ast2ir-rules-v0.1.md,ir-v0.1.md}
│   └── decisions/0001 ... 0006
├── assets/
│   ├── [tracked] README.md
│   └── [tracked] sample-urban-context.png
├── packaging/
│   ├── [tracked] README.md
│   └── [planned] selected installer/package definitions
├── reference/
│   ├── [tracked] README.md
│   └── [local] manuals, templates, fonts, and research inputs
└── artifacts/
    ├── [tracked] README.md
    └── [local] generated HWP/renders/logs
```

Planned entries are not empty scaffolding requirements. In particular, do not
create `upstream/` by hand: `git submodule add` creates each path and gitlink.

## Ownership boundaries

| Boundary | Owns | Does not own |
| --- | --- | --- |
| `apps/md2hwp` | CLI/orchestration; direct Pandoc launch, version check, diagnostics; backend selection | AST meaning, template binding, native document operations |
| `crates/md2hwp-core` | Pandoc JSON decoding, AST2IR handlers, normalization, IR types/I/O/validation | Pandoc process invocation, template inspection, backend lowering |
| `backends/hancom-automation` | C# worker, template binding, COM-specific lowering/mutation/verification | runtime installation and external pins |
| `backends/rhwp` | md2hwp code that consumes the adopted rhwp interface | upstream source history and pin metadata |
| `upstream/pandoc` | inspectable pinned Pandoc source | md2hwp code or the installed runtime executable |
| `upstream/rhwp` | inspectable pinned rhwp source; possible path dependency/CLI build input | md2hwp-specific lowering policy |
| `dependencies/lock.json` | every adopted external identity, exact pin, artifact checksum, and consumer | downloaded payloads or implementation code |
| `rules`, `schemas`, `profiles` | versioned product data and closed contracts | arbitrary executable configuration |
| `tests` | shared fixtures, integration and visual verification | compiler-generated output |
| `tools` | diagnostics, smoke checks, fixture investigation | production conversion |

## Data and process flow

```text
CommonMark / reStructuredText
            │
            ▼
 apps/md2hwp launches pinned Pandoc CLI
            │ Pandoc JSON
            ▼
 md2hwp-core + rules/ast2ir
            │ validated project IR
            ├─────────────────────────────┐
            ▼                             ▼
 backends/hancom-automation       future backends/rhwp
 + profile + template            + same IR/profile meanings
            └──────── output + verification
```

Pandoc JSON may travel through stdout/stdin or a file. Validated project IR is
the only public persistent intermediate document format. Pandoc source is not a
Rust dependency; the installed executable is invoked as a child process.

## External source and version rules

- `upstream/pandoc` uses <https://github.com/jgm/pandoc>.
- `upstream/rhwp` uses <https://github.com/edwardkim/rhwp>.
- The parent Git tree pins each submodule commit; the matching
  `git-submodule` item in `dependencies/lock.json` must name the same revision.
- A Pandoc release executable additionally needs a version, platform artifact
  URL, and SHA-256 in the same lock.
- Hancom's installed runtime and security archive remain entries in the same
  lock; the installer reads only the Hancom entry.
- No current-head, latest-release, or floating branch is adopted merely to
  populate the tree. Compatibility tests choose the pin first.

After the submodules exist, ordinary checkout setup is:

```powershell
git submodule update --init --recursive -- upstream/pandoc upstream/rhwp
```

The supported Pandoc executable may be installed through an official package
or a lock-driven download; `apps/md2hwp` must still reject a version other than
the adopted one. rhwp's future backend decision chooses path dependency versus
CLI; no duplicate source download directory is introduced.

## Language and generated-output conventions

Rust packages use Cargo's conventional `src/` layout within their product
directories and generate `target/`. The C# worker uses its selected .NET
solution/project layout and generates project-local `bin/` and `obj/`. Data
contracts retain semantic roots. There is no repository-wide `src/`, `bin/`,
`dist/`, or `share/` tree.

`reference/` remains ignored local input, `artifacts/` ignored generated
evidence, and `tmp/` ignored cache/scratch space. The pinned Hancom security DLL
is exactly ignored under `dependencies/`. `assets/` may contain explicitly
provisional candidates, but they are excluded from packaging and fixture
promotion until provenance is resolved.

## Open decisions that block planned files

- remote policy and exact Pandoc/rhwp submodule revisions;
- production Hancom worker projects, protocol, and COM interop strategy;
- exact reStructuredText invocation and compatibility fixtures;
- rhwp path-library versus CLI integration and compatibility contract;
- backend invocation envelope, AURI template profile, installer format, and
  visual comparison policy.

The repository is initialized on `main`. Resolve an open decision in
documentation and tests before adding files that claim it has been resolved.
