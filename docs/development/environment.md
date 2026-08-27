# Development environment

## Declared roles

- PowerShell 7 (`pwsh`) runs repository diagnostics and schema smoke tests.
- Windows PowerShell 5.1 (`powershell.exe`) runs the current Hancom Automation
  investigation and dependency setup scripts.
- Rust implements the backend-neutral core and user-facing application. The
  repository pins Rust 1.98.0, uses edition 2024 and Cargo resolver 3, and sets
  the initial MSRV to Rust 1.98.0 (`rust-version = "1.98"`).
- C#/.NET will implement the separate Hancom Automation worker, but the SDK,
  target framework, solution/project files, and COM interop strategy are not
  declared yet.
- Pandoc is the parser boundary, but its supported version and exact invocation
  are not declared yet. It is an external Haskell CLI, invoked directly by the
  application; its eventual executable and source-submodule pins belong in
  `dependencies/lock.json`.

Do not install or pin guessed .NET SDK or Pandoc versions merely to make an
empty workspace appear complete. Record those choices in documentation and
tests first.

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
- the .NET host and runtime 8.0.30 are present, but no .NET SDK is installed;
- `HWPFrame.HwpObject` resolves as a registered COM ProgID;
- the required `FilePathCheckerModuleExample` registry entry and installed DLL
  are absent for the current user.

The COM registration alone is not a ready backend environment. Before opening
any document, the backend must create the COM object and require
`RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample")` to
succeed. Use the explicit dependency-setup command in the root README when
installation is intended.

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
