# Development environment

## Declared roles

- PowerShell 7 (`pwsh`) runs repository diagnostics and schema smoke tests. It
  must not run Hancom COM or the Hancom security-module installer.
- Interactive Windows PowerShell 5.1 x64 in an STA runs the current Hancom
  Automation investigation and dependency setup scripts.
- Rust implements the backend-neutral core and user-facing application. The
  repository pins Rust 1.98.0, uses edition 2024 and Cargo resolver 3, and sets
  the initial MSRV to Rust 1.98.0 (`rust-version = "1.98"`).
- C#/.NET will implement the separate Hancom Automation worker. Repository C#
  work uses the pinned portable .NET 10.0.400 Windows x64 SDK; the investigation
  preview targets `net10.0-windows`. Production project shape, protocol, and
  COM interop strategy remain open.
- Pandoc 3.10.1 is the pinned parser boundary. The official Windows x86_64 zip
  and SHA-256 are declared in `dependencies/lock.json`, and explicit setup
  installs it under `.local/dependencies/pandoc/3.10.1/`.

.NET and Pandoc changes require a new lock, decision, and compatibility check.

## .NET toolchain

Install the official locked SDK without changing the system SDK or `PATH`:

```powershell
pwsh -NoProfile -File .\dependencies\install-dotnet-sdk.ps1
pwsh -NoProfile -File .\tools\development\dotnet.ps1 --info
```

`global.json` disables SDK roll-forward. The wrapper keeps CLI state, NuGet
packages, and the effective user NuGet configuration under ignored
`.local/state/`; the investigation project has no external package dependency.

## Rust toolchain

The root `rust-toolchain.toml` selects the exact compiler and requests a minimal
profile plus `rustfmt`, `clippy`, and `rust-analyzer`. The exact toolchain pin
and MSRV intentionally start at the same release. A future toolchain update
does not raise the MSRV unless `rust-version` and its compatibility checks are
changed deliberately.

Windows prerequisites:

- Visual Studio Build Tools 2022;
- `Microsoft.VisualStudio.Component.VC.Tools.x86.x64`;
- `Microsoft.VisualStudio.Component.Windows11SDK.26100`;
- rustup for the `x86_64-pc-windows-msvc` host.

After installing the native prerequisites and
[rustup](https://www.rust-lang.org/tools/install), reproduce the repository
toolchain with the following command. Open a new terminal first if rustup has
just added `%USERPROFILE%\.cargo\bin` to the user `PATH`.

```powershell
rustup toolchain install 1.98.0-x86_64-pc-windows-msvc `
  --profile minimal `
  --component rustfmt,clippy,rust-analyzer
```

The root toolchain file then keeps repository commands on that exact release.
Verify the complete local baseline with:

```powershell
rustup show active-toolchain
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`Cargo.lock` is committed because the workspace contains the `md2hwp`
application. Build output remains ignored under `target/`.

Primary references:

- [Rust 1.98.0 release](https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/)
- [rustup toolchain files](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
- [rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html)
- [Rust MSVC prerequisites](https://rust-lang.github.io/rustup/installation/windows-msvc.html)

## Reference workstation snapshot

The general environment was observed on 2026-08-26. The Rust and MSVC entries
were installed and verified on 2026-08-27.

- `pwsh` is available (7.6.3);
- Windows PowerShell is available;
- Visual Studio Build Tools 2022 17.14.39 is installed with the declared MSVC
  and Windows SDK components;
- rustup selects `rustc 1.98.0` for `x86_64-pc-windows-msvc`; `cargo 1.98.0`,
  `rustfmt 1.9.0-stable`, `clippy 0.1.98`, and `rust-analyzer 1.98.0` are
  installed, and `%USERPROFILE%\.cargo\bin` is on the user `PATH`;
- `pandoc` is not on `PATH`;
- the pinned portable Pandoc 3.10.1 is installed under the ignored repository
  dependency tree and is invoked by explicit path;
- the system .NET host and runtime 8.0.30 are present, but no system .NET SDK is
  installed; repository C# work uses the installed pinned portable .NET
  10.0.400 SDK, which reports C# 14, runtime 10.0.11, and RID `win-x64`;
- `HWPFrame.HwpObject` resolves as a registered COM ProgID;
- the security-module registration is per-user runtime state and is never
  assumed from repository contents; a prior successful live probe in the
  execution context below records the adopted verification procedure.

## Verified Hancom Automation execution context

The environment was verified on 2026-08-27, and the tracked minimal-fixture
checks were reverified on 2026-08-28:

- Windows identity: the logged-in interactive user that owns the applicable
  HKCU profile; machine names, account names, and SIDs are intentionally not
  recorded in the repository;
- logged-in interactive session (observed as session 1; the numeric session ID
  may change after sign-in or RDP reconnect);
- host: `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`;
- Windows PowerShell `5.1.26100.9168`, `PSEdition=Desktop`, x64, STA;
- COM ProgID `HWPFrame.HwpObject`, launching 32-bit HWP
  `11.0.0.9136` from
  `C:\Program Files (x86)\Hnc\Office 2020\HOffice110\bin\hwp.exe`;
- registry value
  `HKCU\Software\HNC\HwpAutomation\Modules\FilePathCheckerModuleExample`
  targeting a present DLL with the pinned hash; the workstation-specific
  absolute path is discovered live and is not a repository invariant;
- module SHA-256
  `9AC5B97C47AC8AED1E8BCA27A3EEF39411361D8F68C262509F0C40A8F9D21BB6`,
  equal to `dependencies/lock.json`;
- `RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample")=True`;
- `Open=True` for `tests\fixtures\templates\minimal.hwp`,
  both with the HWP window hidden and explicitly visible.

The same environment also passed the documented
[HWP Unicode save/reopen investigation](hwp-unicode-roundtrip.md), preserving
`U+AC00`, the decomposed sequence `U+1100 U+1161`, and the emoji sequence
`U+1F3D9 U+FE0F` after TEXT-transport decoding. The emoji's storage
preservation is distinct from its missing-glyph appearance in the current PNG
render.

The C# investigation child process was adopted on 2026-08-28 for the preview
program only. The exact Windows PowerShell host above launched the pinned .NET
10.0.400 x64 runtime as the same logged-in interactive identity and passed its
open-only `probe`: module registration, HWP open, and input-template hash
preservation were all true. Its `render` mode then copied the minimal HWP
fixture, applied the six diagnostic text operations and one figure operation
from `examples/ir-v0.1.json`, saved and reopened the copy, found its text
marker and one added picture, and confirmed that the source template was
unchanged. No HWP process remained after either operation.

On 2026-08-29, a style-aware render bound the example to the fixture's unique
native AURI style names: `본문`, the six heading levels, `박스내용`,
`표그림_캡션`, and `출처 및 하단설명`. Save/reopen checks found every appended
text paragraph under the expected native style, found one added picture, and
confirmed that the template hash was unchanged. A three-page PNG export showed
the native heading, body, caption, and source typography. It also showed why
style names alone are not the complete lowering contract: the box control,
native list markers, and caption automatic numbering still require
backend-specific structure or actions.

On 2026-08-30, nested strong/emphasis runs were rendered with the bound native
style as their base. Save/reopen verification resolved every direct HWPML text
run through its character-shape definition and matched the expected effective
bold/italic flags. The page export showed the marked text while retaining the
base font and size. A rejected approach is also part of the environment result:
`GetDefault("CharShape", ...)` returned a zeroed full shape at the insertion
caret, so executing it as though it were a partial bold/italic patch produced
zero height, font, ratio, and relative-size fields. The adopted investigation
path instead uses the dedicated bold/italic transitions and restores the native
style's HWPML-derived base state after each formatted line.

The same reference context then passed a minimal-fixture native box-clone
render. The investigation selected the unique root `본문` paragraph containing
one inline table, one internal `박스내용` paragraph, and one internal
`출처 및 하단설명` placeholder. It captured that root as an in-memory native HWP
`saveblock` and inserted it with `SetTextFile(..., "HWP", "insertfile")`, without
using the system clipboard. Each box remained one preview operation and its
logical lines, including an empty line and a trailing empty line, reopened as
`LINEBREAK` children inside the same content paragraph. Verification observed
exactly one added table, no added picture or automatic-number control, an
unchanged source prototype, and an unchanged template hash. HWP regenerated
native `InstId` and shape `ZOrder` values during insertion, including values in
existing roots. It also recalculated the box-layout `LastWidth` and `SIZE.Height`
as its logical line count changed. Structural comparison excludes
those four known instance/layout fields, normalizes only the content payload,
and compares the remaining prototype XML. Direct HWPML2X insertion was rejected
because it inserted no control in this tested path.

A separate two-box IR render used content identical to the template prototype,
exercising repeated insertion when adjacent roots serialize alike. It reported
two box operations and two boxes added, then reopened both clones with their
exact logical lines and verified the unchanged source prototype and template hash.
The two-page PNG export was 992 by 1403 pixels per page and showed both native
blue-border boxes with their content-dependent heights. No HWP process remained
after rendering or export.

The exported second page showed the cloned blue-border box and native internal
style. It also retained the prototype's `출처:` placeholder as template
decoration; verbatim-block IR has no corresponding source field, so production
lowering must resolve that policy before adoption. This selector intentionally
does not bind the full AURI reference document, whose box has a different root
style and four internal `박스내용` paragraphs. Native list markers and caption
automatic numbering remained unresolved after this box probe.

The same 2026-08-30 reference context then passed the minimal-fixture native
figure-caption clone. The unique source was a root `표그림_캡션` paragraph with
literal `[그림 ` text, one decimal `AUTONUM NumberType=Figure` control, and the
literal `] 스타일 대응 예시` suffix. The investigation captured a native HWP
`saveblock`, inserted it with `SetTextFile(..., "HWP", "insertfile")`, and
replaced only the human suffix. The reopened result retained the caption style,
control shape, IR strong/emphasis character runs, and one added Figure automatic
number per inserted picture. The ordinary full-document HWPML root remained the
structural comparison source. HWP 2020 exposed the selected block's inspection
HWPML differently: the caption content followed `SECDEF` inside its
section-definition `TEXT`. Preflight therefore verifies the exact `CHAR`
siblings around that unique control instead of pretending the saveblock is an
ordinary caption root.

Inserting the native block also materialized a just-inserted picture's derived
`ROTATIONINFO CenterX` and `CenterY` values from zero. Caption insertion
identity excludes only that lazy pair; the picture geometry and transform stay
in the root comparison. A separate two-figure fixture used captions identical
to the source prototype. It reported two pictures and two Figure controls
added, reopened with exact caption content, and its 992-by-1403 PNG pages showed
the new native labels in sequence as `[그림 1]` and `[그림 2]`. The template
hash stayed unchanged, and no HWP process remained after render or export.

This remains a minimal-fixture result. The full AURI reference document's
observed caption is nested in a picture object rather than represented by this
root-paragraph shape, so the investigation selector rejects it instead of
guessing. A production profile must define and test that binding. Native list
markers are the remaining visible structural gap in this preview.

In this C# late-bound COM context, HWP 2020 returned a non-null COM object from
`InsertPicture`, rather than the Boolean result assumed by the initial
implementation. The preview accepts either Boolean true or a COM object as the
immediate result, then treats the saved-and-reopened picture-count check as the
structural proof of insertion. This observation does not settle the production
worker's COM interop strategy or template lowering contract.

All Hancom COM investigation, security-module installation or re-registration,
and backend verification on this workstation must run under the same Windows
identity/profile in a logged-in interactive session using that exact Windows
PowerShell x64 host. A sandbox account, another user, PowerShell 7, x86 Windows
PowerShell, a service, or a noninteractive session is not an equivalent
environment. In particular, HKCU and `%LOCALAPPDATA%` observations belong to
the invoking identity; do not use another identity's results to declare the
module present or absent.

From the repository root, install or verify the module with:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\dependencies\install-hancom-security-module.ps1
```

Then require the open-only probe to pass in a fresh process:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\test-hwp-open.ps1 `
  -File .\tests\fixtures\templates\minimal.hwp
```

Add `-Visible` only when a human-visible launch check is useful. The probe
never saves the document. It requires the pinned DLL hash, `RegisterModule`
success, and `Open` success, then discards and closes the document and COM
object.

### Why the earlier environment result was wrong

The 2026-08-27 investigation initially mixed several non-equivalent contexts:

1. `tools/development/show-environment.ps1` was first run under a sandbox
   identity rather than the logged-in desktop user. The sandbox identity's
   empty HKCU was incorrectly reported as the desktop user's missing
   registration.
2. The original installer was then run with PowerShell 7 x64. It copied the
   DLL and overwrote the HKCU value before COM validation; when
   `RegisterModule` returned false, it left those changes behind. The managed
   DLL created by that run and the pre-existing system-directory DLL had the
   same pinned hash, but different deployment paths.
3. Windows PowerShell 5.1 could not run the old installer without explicit
   parameters because `$PSScriptRoot` was used in a parameter default before it
   was populated. Alternate x86/in-process COM experiments either failed
   registration or crashed on `Open`; they are not adopted environments.
4. Some interrupted probes left hidden automation HWP processes temporarily
   alive. A fresh, isolated Windows PowerShell 5.1 x64 STA process with the
   working pinned registry target produced the reproducible successful result
   recorded above.

### Automated safeguards and remaining coverage

The open probe enforces the verified host, bitness, apartment, and interactive
session. The installer must still be launched manually in that documented
context, but it does not claim that process metadata can prove which HKCU hive
is visible. Both reject pre-existing HWP processes, verify the registered DLL
against the locked SHA-256, require `RegisterModule=true`, and clean up the HWP
process they create. The open probe additionally requires `Open=true` and never
saves. Before any download or write, the installer preserves any valid pinned
REG_SZ registration as `Action=AlreadyValid`, regardless of install directory.
If Hancom rejects that otherwise-valid registration it fails without mutation.
If registration needs repair, the installer reuses the ignored managed DLL
without downloading when its hash matches the content pin. Otherwise, a
fallback installation downloads the official archive. Both paths snapshot and
restore the previous registry value/type, and the download path also restores
the previous managed file if validation fails.

Live verification remains an explicit workstation operation, never a build
side effect. The current repeatable sequence is: wrong-host rejection, installer
`AlreadyValid` or transactional install, a fresh open-only probe, zero remaining
HWP processes, the contract smoke test, and Rust workspace tests.

Before relying on the new-install rollback path in production, add a
failure-injection test harness with fake file, registry, and COM adapters. It
must fail after every mutation boundary and assert byte-for-byte file recovery,
registry value/type recovery, preservation of unrelated values, idempotent
second runs, and combined reporting when rollback itself fails. A separate
real-registry/no-COM test may use only a GUID-named temporary HKCU subtree and
must delete exactly that subtree after verification.

The COM registration alone is not a ready backend environment. Before opening
any document, the backend must create the COM object and require
`RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample")` to
succeed. Use the verified command above when installation is intended.

Run non-mutating discovery with:

```powershell
pwsh -NoProfile -File .\tools\development\show-environment.ps1
```

Run the current smallest repeatable test with:

```powershell
pwsh -NoProfile -File .\tools\smoke\test-contracts.ps1
```

This validates the product-owned AST-to-IR ruleset, the single external
dependency lock, the accepted IR example, and explicit IR rejections. It does
not run Pandoc or instantiate Hancom COM.
