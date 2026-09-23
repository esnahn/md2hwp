# md2hwp

`md2hwp` is intended to convert an explicit shared subset of Pandoc-normalized
CommonMark and reStructuredText into Korean Hangul documents by editing a
required HWP/HWPX template. The repository now contains its first deterministic
CommonMark-to-validated-IR slice, but not yet a production HWP/HWPX converter.

Repository boundaries and placement rules are documented in
[`docs/design/repository-layout.md`](docs/design/repository-layout.md).
Reference-workstation prerequisites and current gaps are recorded in
[`docs/development/environment.md`](docs/development/environment.md).

The accepted next design is **validated IR + an authored HWP template** as the
render inputs, with role declarations and reusable style/structure samples
inside the template instead of a separately maintained profile JSON. See
[ADR 0008](docs/decisions/0008-template-owned-bindings.md). This direction is
implemented for the `minimal-1` investigation path: the tracked
[`minimal-tagged-v1.hwp`](tests/fixtures/templates/minimal-tagged-v1.hwp) and
`render-tagged` command accept IR and a tagged HWP without an external profile.
See the [usage guide and verification evidence](docs/development/minimal-tagged-template.md).
The existing `render` command retains its investigation profile. General
template support and the production worker interface remain open.

For the current CommonMark → tagged HWP investigation, run one command after
the documented workstation prerequisites are installed:

```powershell
pwsh -NoProfile -File .\apps\md2hwp\convert.ps1 `
  -InputPath .\examples\report-workflow-v0.2.md `
  -Template .\tests\fixtures\templates\minimal-tagged-v1.hwp `
  -Output .\artifacts\my-report.hwp
```

See the [single-command workflow](docs/development/single-command-workflow.md)
for its supported scope, failure behavior, and report fixture.

With the published `md2hwp.exe`, `md2hwp-backend.exe` and tagged `template.hwp`
in one folder, the shorthand commands are:

```powershell
.\md2hwp.exe .\원고.md
.\md2hwp-backend.exe .\원고.ir.json
```

The first writes `원고.ir.json` and `원고.result.hwp` beside the Markdown file;
the second renders an existing IR to `원고.result.hwp`. An optional second
argument selects the HWP output path. Shorthand replaces generated IR after
validation and replaces an existing HWP only after rendering and verification
succeed. Failed rendering preserves the previous HWP and retains the new IR.
Source manuscripts and templates cannot be selected as replacement outputs.
Shorthand image resources resolve inside the source/IR folder. The existing
subcommands and `--template`, `--worker`, `--dotnet` and other options remain
available through their original command forms. See
[runtime prerequisites and deployment](docs/development/dotnet-runtime-launch.md).

The repository is organized by product responsibility:

- `apps/`, `crates/`, and `backends/` contain md2hwp code boundaries;
- `dependencies/lock.json` records all adopted external pins, while `rules/`,
  `schemas/`, and `profiles/` contain product data contracts;
- `upstream/pandoc` and `upstream/rhwp` will be pinned Git submodules once the
  repository and compatible revisions are selected;
- `tests/`, `tools/`, `reference/`, and `artifacts/` separate verification,
  repository tooling, local inputs, and generated evidence.

See the [file-level repository map](docs/design/repository-layout.md) for the
current files and the implementation files that are intentionally deferred
until their toolchain decisions are made.

The canonical AST-to-IR policy is the versioned product data in
[`rules/ast2ir/ir-v0.2.json`](rules/ast2ir/ir-v0.2.json), specified in
[`docs/specifications/ir-v0.2.md`](docs/specifications/ir-v0.2.md).

## Project IR

New normalization emits [IR 0.2](docs/specifications/ir-v0.2.md), adding optional
box source metadata. The original 0.1 schema and files remain readable under
their original contract.

The first closed, backend-neutral serialization contract is now defined in:

- [`docs/specifications/ir-v0.1.md`](docs/specifications/ir-v0.1.md) — normative
  semantics and policies;
- [`schemas/ir-v0.1.schema.json`](schemas/ir-v0.1.schema.json) — JSON Schema
  Draft 2020-12 contract;
- [`examples/ir-v0.1.json`](examples/ir-v0.1.json) — valid serialized example.
- [`examples/ir-v0.1-rejected-page-break.json`](examples/ir-v0.1-rejected-page-break.json)
  — intentionally invalid example proving that page breaks are outside v0.1.
- [`examples/ir-v0.1-rejected-soft-break.json`](examples/ir-v0.1-rejected-soft-break.json)
  — intentionally invalid because source soft breaks normalize to IR spaces.

In PowerShell 7 (`pwsh`), run the repeatable schema smoke test with:

```powershell
pwsh -NoProfile -File .\tools\smoke\test-contracts.ps1
```

The script validates the AST-to-IR ruleset and the single external dependency
lock, requires the accepted IR example to validate, and requires
both page-break and soft-break IR examples to be rejected. Windows PowerShell
5.1 (`powershell.exe`) does not provide `Test-Json`; it is used separately by
Hancom Automation scripts.

The Rust core additionally implements the closed IR types, strict decoding,
semantic validation, and validated-only serialization. Run its tests with:

```powershell
cargo test -p md2hwp-core
```

Safe asset resolution, template style/role mappings, and backend capabilities
remain later preflight responsibilities.

## CommonMark to IR

Install the locked portable Pandoc and convert the visual comparison example:

```powershell
pwsh -NoProfile -File .\dependencies\install-pandoc.ps1
cargo run -p md2hwp -- md2ir `
  --from commonmark `
  --input .\examples\commonmark-v0.1.md `
  --output .\artifacts\commonmark-v0.1.ir.json
```

The source example is [CommonMark](examples/commonmark-v0.1.md), and its golden
result is [expected IR](examples/commonmark-v0.1.expected.ir.json), with only the
envelope upgraded to 0.2 for new runs. Figure and box source normalization now
uses [the shared Markdown convention](docs/development/markdown-object-sources.md).
The direct-IR figure in `ir-v0.1.json` is
intentionally absent from this pair. Run the end-to-end comparison with:

```powershell
pwsh -NoProfile -File .\tools\smoke\test-commonmark-to-ir.ps1
```

The observed Pandoc reader behavior for normalization-sensitive Unicode is
recorded in
[`docs/development/pandoc-unicode-normalization.md`](docs/development/pandoc-unicode-normalization.md).

An existing Pandoc JSON file can enter at the parser boundary without installing
or launching Pandoc:

```powershell
cargo run -p md2hwp -- md2ir `
  --from pandoc-json `
  --input .\tests\fixtures\pandoc-json\commonmark-v0.1.json `
  --output .\artifacts\pandoc-json-v0.1.ir.json
```

Run its direct-input smoke test with:

```powershell
pwsh -NoProfile -File .\tools\smoke\test-pandoc-json-to-ir.ps1
```

## C# Hancom investigation preview

The first C# investigation program reads validated IR and produces an explicit
preview plan before any COM call. It uses the pinned repository-local .NET SDK:

```powershell
pwsh -NoProfile -File .\dependencies\install-dotnet-sdk.ps1
pwsh -NoProfile -File .\tools\smoke\test-hancom-ir-preview-plan.ps1
```

The manually gated HWP open-only and diagnostic render commands are documented
in
[`tools/investigation/hancom-automation/ir-preview/README.md`](tools/investigation/hancom-automation/ir-preview/README.md).
They are investigation aids, not template-profile or backend lowering.

## Hancom Automation security module

Download the Automation security module from the
[official Hancom guide](https://developer.hancom.com/hwpautomation), unpack it,
and follow its included registration instructions in the Windows account that
runs Hancom. Register `FilePathCheckerModuleExample` as REG_SZ with the DLL's
absolute path under `HKCU\Software\HNC\HwpAutomation\Modules`.

The application does not download, install, register, or hash-pin this module.
It checks the registered file and requires `RegisterModule=true` before opening
a document. The old installer script is retired and only prints guidance.
Agents must not install the module or mutate the registration.

After setup, close open Hancom documents and run the canonical open-only probe
in the verified interactive Windows PowerShell 5.1 x64 STA context:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\test-hwp-open.ps1 `
  -File .\tests\fixtures\templates\minimal.hwp
```

The probe never saves. It must report `RegisterModule=True` and `Open=True`.
Observed DLL hashes and process module snapshots are diagnostics, not version
gates. See [environment](docs/development/environment.md).

User-facing Pandoc setup is `md2hwp setup-pandoc`: prefer the development
baseline download, otherwise the latest official stable release, under
`%LOCALAPPDATA%\md2hwp\pandoc\`. Installed Pandoc versions are accepted based on
their JSON contract, not an exact release number. See
[runtime dependency policy](docs/development/runtime-dependencies.md).

## Pandoc and rhwp source

Pandoc is consumed as an external CLI rather than a Rust crate. Its process
invocation stays in `apps/md2hwp`; AST decoding and normalization remain in
`crates/md2hwp-core`. Use the repository's locked portable installer:

```powershell
pwsh -NoProfile -File .\dependencies\install-pandoc.ps1
```

Source inspection will use `upstream/pandoc` and `upstream/rhwp` Git
submodules. Their source revisions have not been selected; this is separate
from the adopted Pandoc 3.10.1 release executable already pinned in
`dependencies/lock.json`. See
[`dependencies/README.md`](dependencies/README.md) for the retrieval and pin
contract.
