# Installed .NET runtime and single-file worker

The Hancom investigation now uses a framework-dependent single-file C# EXE. Distribution
does not include .NET. Users install **.NET Runtime 10.0, Windows x64**, from
<https://dotnet.microsoft.com/ko-kr/download/dotnet/10.0>. An SDK is unnecessary;
an SDK or Desktop Runtime installation is also usable if it provides the same
Microsoft.NETCore.App runtime. Hancom and its security module remain separate
prerequisites.

Rust launches `md2hwp-backend.exe render-tagged ...` in a separate process.
It does not host the CLR. The worker DLL, `.runtimeconfig.json`, and `.deps.json`
are bundled inside the EXE; only the EXE is deployed for the C# component.
This adopts deployment/runtime checking only, not a general production protocol.
The original investigation modes retain their development DLL runner.

Build and collect the three deployment files with:

```powershell
pwsh -NoProfile -File .\tools\development\build.ps1
```

The default is Release: `target/release/` receives `md2hwp.exe`,
`md2hwp-backend.exe` and `template.hwp`. `-Configuration Debug` selects
`target/debug/`. Only these three files need to be copied for deployment.
The tracked `minimal-tagged-v1.hwp` keeps its original filename; publication
copies it as `template.hwp`. Rebuilding refreshes that generated copy.
Plain `cargo build` builds Rust only; use this script to assemble the bundle.

To publish the backend separately with the tracked profile:

```powershell
pwsh -File .\tools\development\dotnet.ps1 publish `
  .\tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj `
  --configuration Release -p:PublishProfile=FrameworkDependent
```

The project's `bin/Release/net10.0-windows/win-x64/publish/` receives
`md2hwp-backend.exe` and `template.hwp` (the former EXE build name was `Md2Hwp.HancomIrPreview.exe`).
No runtime, PDB, trimming, or Native AOT is included.

The supported worker runtimeconfig declares Microsoft.NETCore.App **10.0.0**.
The launcher accepts installed stable 10.0.x patches and requests `LatestPatch`
roll-forward, with prerelease roll-forward disabled. The publish smoke checks
the runtime configuration against this contract.

Host discovery tries `DOTNET_ROOT_X64`, `DOTNET_ROOT`, the standard Program Files
installation, then absolute PATH entries. `--dotnet` selects one explicit host
without fallback. Each candidate must be a Windows x64 PE executable and report
a stable Microsoft.NETCore.App 10.0.x through `--list-runtimes`. An x86 installation,
another major/minor, or a preview alone does not satisfy the prerequisite.
The published apphost uses `AppHostDotNetSearch=EnvironmentVariable`. Rust sets
both `DOTNET_ROOT_X64` and `DOTNET_ROOT` to the checked host's directory, ensuring
the EXE uses that runtime rather than a different global installation.

Missing or incompatible runtime produces a Korean console diagnostic with the
official installation URL, exits unsuccessfully, and never launches the worker.
The CLI does not show a GUI dialog or install anything automatically. Runtime
presence does not imply that Hancom or security registration is ready.

```powershell
.\target\debug\md2hwp.exe check-runtime
.\target\debug\md2hwp.exe check-runtime --dotnet .\.local\dependencies\dotnet\10.0.400\dotnet.exe
```

`render-hwp` requires `--ir` and `--output`. Optional `--worker` and `--template`
default to `md2hwp-backend.exe` and `template.hwp` beside the Rust executable,
independent of the working directory. Explicit paths override these defaults;
missing files fail with their resolved paths. `--dotnet` is also optional.
Standalone `md2hwp-backend.exe render-tagged` similarly defaults to `template.hwp`
beside the backend executable. Relative explicit paths and manuscript resources
still resolve from the working directory. Only render-tagged gains this default;
template authoring and legacy investigation modes retain explicit inputs.

```powershell
.\md2hwp.exe render-hwp --ir .\document.ir.json --output .\result.hwp
.\md2hwp-backend.exe render-tagged --ir .\document.ir.json --output .\result.hwp
```

Both EXEs also accept positional shorthand without a subcommand:

```powershell
.\md2hwp.exe .\원고.md [결과.hwp]
.\md2hwp-backend.exe .\원고.ir.json [결과.hwp]
```

Brackets denote an optional argument, not literal command text. The Rust form
always writes `원고.ir.json` beside the Markdown source, then renders HWP. The
default HWP name is `원고.result.hwp`, also beside the source; the backend strips
the complete `.ir.json` suffix before adding `.result.hwp`. An explicit output
is relative to the caller's working directory. Shorthand replaces existing IR
after validation, using a temporary file and atomic publication. Tagged rendering
(both positional and option-based) replaces existing HWP only after temporary
document generation, reopen and verification succeed. Failures preserve the
previous HWP; a successfully normalized new IR remains available for retry.
Source/template output aliases are rejected. The standalone `md2ir` command
retains its explicit `--force` option; shorthand enables IR replacement itself.

Positional rendering uses the source/IR directory as the image resource root,
independent of the caller's working directory. Images must stay within that
directory. Existing option-based modes retain their cwd resource root and
explicit override behavior. Positional syntax takes only input and optional
output; use the original option-based commands for additional configuration.

Rust validates IR before launching C#, protects input/template paths, and
leaves template binding, COM safeguards, save/reopen verification, and cleanup to
the existing worker. Worker failure is reported as a nonzero application exit.

The single-command wrapper uses this path through the established Windows
PowerShell 5.1 interactive host. Default discovery uses installed runtimes;
development can explicitly select the pinned local SDK's runtime:

```powershell
pwsh -File .\apps\md2hwp\convert.ps1 `
  -InputPath .\examples\report-workflow-v0.2.md `
  -Template .\tests\fixtures\templates\minimal-tagged-v1.hwp `
  -Output .\artifacts\single-exe-report.hwp `
  -RuntimeHostPath .\.local\dependencies\dotnet\10.0.400\dotnet.exe
```

The pinned SDK is still needed to **build** C# in this repository. End users of
prebuilt binaries need only the runtime. The wrapper remains an investigation
entry point, not a finished distributable application. `-SkipBuild` requires the
published EXE for the selected configuration.

## Schemas and remaining repository dependencies

**No separate schema file is needed at runtime.** Rust embeds IR 0.1/0.2 JSON
Schemas and AST2IR schema/rules at compile time. C# checks its accepted IR shape
in code and does not read JSON Schema files. The earlier statement that
repository schemas were runtime inputs was incorrect.

`render-tagged` no longer discovers a repository or reads `dependencies/lock.json`.
Its working directory is the resource root for relative IR image paths. Legacy
fixture/profile modes still use repository discovery. Security-module registration
is checked without a fixed hash. Pandoc is a separate downloadable prerequisite
for Markdown input. See [runtime dependency policy](runtime-dependencies.md).

## Verification, 2026-09-23

- Rust application tests (9), clippy, and conversion rejection
  smoke passed.
- `tools/smoke/test-runtime-launch.ps1` copies only the published EXE into an
  empty directory and runs `runtime-info` without sidecar JSON. It checks the
  actual runtime/architecture, generated runtime configuration, missing-runtime
  guidance for both checking and rendering, no HWP output, and template preservation.
  `runtime-info` runs before repository discovery and never starts COM.
- The canonical open-only probe passed in the recorded interactive environment.
- Live EXE output is `artifacts/single-exe-report.hwp`, using the existing report
  fixture. Save/reopen and template preservation passed; all four exported page
  PNGs exactly match `artifacts/report-workflow-v0.2-pages`. No HWP process remained.
- The successful run explicitly used the pinned SDK's local runtime. Default
  system discovery correctly reports no compatible 10.0 runtime on this machine.
