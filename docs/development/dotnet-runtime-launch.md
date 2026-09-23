# Installed .NET runtime and Rust worker launch

The Hancom investigation now uses a framework-dependent C# worker. Distribution
does not include .NET. Users install **.NET Runtime 10.0, Windows x64**, from
<https://dotnet.microsoft.com/ko-kr/download/dotnet/10.0>. An SDK is unnecessary;
an SDK or Desktop Runtime installation is also usable if it provides the same
Microsoft.NETCore.App runtime. Hancom and its security module remain separate
prerequisites.

Rust launches `dotnet.exe worker.dll render-tagged ...` in a separate process.
It does not host the CLR or load the managed DLL into the Rust process. This
adopts deployment/runtime checking only, not a general production worker protocol.
The investigation still needs repository schemas and assets and a repository
working directory. Deploy the DLL, `.runtimeconfig.json`, and `.deps.json`
together. Self-contained builds from the size experiment are not this contract.

The supported worker runtimeconfig declares Microsoft.NETCore.App **10.0.0**.
The launcher accepts installed stable 10.0.x patches and requests `LatestPatch`
roll-forward, with prerelease roll-forward disabled. A changed framework or
minimum version is rejected until this contract is updated and tested.

Host discovery tries `DOTNET_ROOT_X64`, `DOTNET_ROOT`, the standard Program Files
installation, then absolute PATH entries. `--dotnet` selects one explicit host
without fallback. Each candidate must be a Windows x64 PE executable and report
a stable Microsoft.NETCore.App 10.0.x through `--list-runtimes`. An x86 installation,
another major/minor, or a preview alone does not satisfy the prerequisite.
The same resolved host is used for inspection and execution.

Missing or incompatible runtime produces a Korean console diagnostic with the
official installation URL, exits unsuccessfully, and never launches the worker.
The CLI does not show a GUI dialog or install anything automatically. Runtime
presence does not imply that Hancom or security registration is ready.

```powershell
.\target\debug\md2hwp.exe check-runtime
.\target\debug\md2hwp.exe check-runtime --dotnet .\.local\dependencies\dotnet\10.0.400\dotnet.exe
```

`render-hwp` accepts `--worker`, `--ir`, `--template`, `--output`, and optionally
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
  -Output .\artifacts\runtime-launch-report.hwp `
  -RuntimeHostPath .\.local\dependencies\dotnet\10.0.400\dotnet.exe
```

The pinned SDK is still needed to **build** C# in this repository. End users of
prebuilt binaries need only the runtime. The wrapper remains an investigation
entry point, not a finished distributable application.

## Verification, 2026-09-23

- Rust application tests (10), clippy, contract smoke, and conversion rejection
  smoke passed.
- `tools/smoke/test-runtime-launch.ps1` checks a compatible local runtime and
  missing-runtime guidance for both checking and rendering, with no HWP output
  and no template changes.
- The canonical open-only probe passed in the recorded interactive environment.
- `artifacts/runtime-launch-report.hwp` was generated through Rust runtime
  checking and the C# DLL, then reopened successfully. All four page PNG hashes
  equal `artifacts/report-workflow-v0.2-pages`; the previously documented short
  trailing paragraph line on page four is unchanged.
- The successful run explicitly used the pinned SDK's local runtime. Default
  system discovery correctly reports no compatible 10.0 runtime on this machine.
