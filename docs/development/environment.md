# Development environment

## Declared roles

- PowerShell 7 (`pwsh`) runs repository diagnostics and schema smoke tests. It
  must not run Hancom COM or the Hancom security-module installer.
- Interactive Windows PowerShell 5.1 x64 in an STA runs the current Hancom
  Automation investigation and dependency setup scripts.
- Rust implements the backend-neutral core and user-facing application. The
  repository pins Rust 1.98.0, uses edition 2024 and Cargo resolver 3, and sets
  the initial MSRV to Rust 1.98.0 (`rust-version = "1.98"`).
- C#/.NET will implement the separate Hancom Automation worker, but the SDK,
  target framework, solution/project files, and COM interop strategy are not
  declared yet.
- Pandoc 3.10.1 is the pinned parser boundary. The official Windows x86_64 zip
  and SHA-256 are declared in `dependencies/lock.json`, and explicit setup
  installs it under `.local/dependencies/pandoc/3.10.1/`.

Do not install or pin a guessed .NET SDK merely to make an empty backend
boundary appear complete. Pandoc changes require a new lock, decision, and
compatibility fixture.

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
- the .NET host and runtime 8.0.30 are present, but no .NET SDK is installed;
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
both `U+AC00` and the decomposed sequence `U+1100 U+1161` after TEXT-transport
decoding.

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
A fallback installation snapshots and restores the previous registry value/type
and managed file if validation fails.

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
