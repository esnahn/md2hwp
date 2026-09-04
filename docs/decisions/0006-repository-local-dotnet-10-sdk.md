# ADR 0006: Repository-local .NET 10 SDK

## Status

Accepted on 2026-08-28 for C# investigation and the future Hancom Automation
worker toolchain. This does not decide the worker process protocol or COM
interop strategy.

## Context

The reference workstation has the .NET 8 runtime but no .NET SDK. Its system
.NET Framework compiler supports only C# 5 and is not an appropriate baseline
for new worker code. The C# environment must also be reproducible without a
machine-wide SDK installation.

## Decision

- Pin the official .NET 10.0.400 Windows x64 SDK ZIP and Microsoft SHA-512 in
  `dependencies/lock.json`.
- Use .NET 10 because it is the current active LTS line. Target Windows and x64
  explicitly for Hancom Automation investigation projects.
- Install the portable SDK under the ignored
  `.local/dependencies/dotnet/10.0.400` directory. Keep CLI state and NuGet
  caches under ignored `.local/state/`.
- Pin SDK selection in `global.json` with roll-forward disabled. Invoke it
  through `tools/development/dotnet.ps1`; do not depend on `dotnet` from PATH.
- Investigation projects may use modern SDK-style projects, nullable analysis,
  deterministic builds, and warnings as errors. A production worker project is
  still deferred until its invocation and COM interop decisions are resolved.

## Consequences

The SDK archive is downloaded only by an explicit setup command and verified
before extraction. The payload and build/cache output are not tracked. Source,
project files, lock metadata, and setup scripts are tracked.

Official release metadata:

- <https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/10.0/releases.json>
- <https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core>
