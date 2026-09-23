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

Use `dependencies/install-hancom-security-module.ps1`; manual download is not
normally needed. This is the supported setup path for the repository. The
installer first checks the existing REG_SZ registration, absolute DLL path,
locked hash, and Hancom `RegisterModule` result. If that registration is valid,
it returns `Action=AlreadyValid` without downloading, moving, or rewriting
anything.

If the registered path, file, and hash are pinned but Hancom rejects
`RegisterModule`, the installer fails without changing installation state. Only
a missing or malformed registration, missing registered file, or hash mismatch
enters repair. The installer first checks the ignored repository-local
`dependencies/FilePathCheckerModuleExample.dll` and reuses it without
downloading when its hash matches the content pin. Otherwise, it downloads the
official `보안모듈(Automation).zip` from the URL in `dependencies/lock.json`,
verifies the pinned DLL hash, and replaces the local copy. Both repair paths
register only the named HKCU value and validate it with Hancom. If validation
fails, the installer restores the previous registry value; the download path
also restores the previous local DLL.

This is a required manual workstation step. The logged-in desktop user whose
HWP Automation process will use the registration must open their own Windows
PowerShell 5.1 window and run the command below from the repository root. Do
not delegate installation or re-registration to an agent command runner,
including an approved non-default run. Run it yourself and return the complete
output when an agent requests setup.

The default Codex sandbox runs under a distinct Windows identity and therefore
sees that identity's HKCU. Environment variables such as `USERNAME` and
`USERPROFILE` can still name the desktop profile, so they do not establish the
process identity; the Windows process token is authoritative. Registering the
sandbox HKCU would not configure HWP Automation for the logged-in desktop user.

Run:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\dependencies\install-hancom-security-module.ps1
```

Then inspect both registry views in that same user-opened PowerShell window:

```powershell
& C:\Windows\System32\reg.exe query "HKCU\Software\HNC\HwpAutomation\Modules" /v FilePathCheckerModuleExample /reg:32
& C:\Windows\System32\reg.exe query "HKCU\Software\HNC\HwpAutomation\Modules" /v FilePathCheckerModuleExample /reg:64
```

The downloaded binary is intentionally not tracked in version control. Its
source is the
[Hancom Developer HWP Automation page](https://developer.hancom.com/en-us/hwpautomation)
and its
[direct official archive link](https://github.com/hancom-io/devcenter-archive/raw/main/hwp-automation/%EB%B3%B4%EC%95%88%EB%AA%A8%EB%93%88%28Automation%29.zip).
The installer does not open a document, and the DLL is never committed.

A fallback installation created by this script points the HKCU registration
directly to that working-tree path. Moving or deleting the repository, or
running a cleanup that removes ignored files such as `git clean -fdx`,
invalidates that registration. Re-run the installer from the repository's new
or restored location before the next HWP Automation operation.

The expected DLL SHA-256 and official URL are pinned in
[`dependencies/lock.json`](dependencies/lock.json),
which the installer reads. If Hancom replaces the official archive, review the
new binary before updating the lock. The installer is explicit and is never
run as a build side effect.

After setup, close any existing HWP processes and verify the open-only probe in
the same logged-in Windows identity. You can run it from the same user-opened
PowerShell window. An agent may instead run this probe in an explicitly
approved non-default context after confirming that its output reports the same
user, interactive session, and verified PowerShell host. A default-sandbox
result is not workstation verification.

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\test-hwp-open.ps1 `
  -File .\tests\fixtures\templates\minimal.hwp `
  -Visible
```

The probe never saves the document. It must report both `RegisterModule=True`
and `Open=True`. `ModuleLoadObservation` is diagnostic only: x64 .NET process
snapshots can omit modules loaded by the x86 HWP process, so a module that is
not observed there does not fail the probe. Further context and safeguards are
documented in
[`docs/development/environment.md`](docs/development/environment.md#verified-hancom-automation-execution-context),
but historical output there does not replace this live check for the current
workstation state.

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
