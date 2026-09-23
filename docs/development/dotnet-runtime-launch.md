# Installed .NET runtime and single-file worker

The Hancom investigation now uses a framework-dependent single-file C# EXE. Distribution
does not include .NET. Users install **.NET Runtime 10.0, Windows x64**, from
<https://dotnet.microsoft.com/ko-kr/download/dotnet/10.0>. An SDK is unnecessary;
an SDK or Desktop Runtime installation is also usable if it provides the same
Microsoft.NETCore.App runtime. Hancom and its security module remain separate
prerequisites.

Rust launches `worker.exe render-tagged ...` in a separate process.
It does not host the CLR. The worker DLL, `.runtimeconfig.json`, and `.deps.json`
are bundled inside the EXE; only the EXE is deployed for the C# component.
This adopts deployment/runtime checking only, not a general production protocol.
The original investigation modes retain their development DLL runner.

Publish with the tracked profile:

```powershell
pwsh -File .\tools\development\dotnet.ps1 publish `
  .\tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj `
  --configuration Release -p:PublishProfile=FrameworkDependent
```

The project's `bin/Release/net10.0-windows/win-x64/publish/` contains only
`Md2Hwp.HancomIrPreview.exe`: **499,366 bytes** in the measured Release build.
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

`render-hwp` accepts `--worker <worker.exe>`, `--ir`, `--template`, `--output`, and optionally
`--dotnet`. It validates IR before launching C#, refuses an existing output, and
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
