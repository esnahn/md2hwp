# Hancom Automation backend

`launcher.rs` is the Rust integration for the existing investigation worker.
It checks an installed .NET 10 x64 runtime before starting the C# single-file EXE in a
separate process. See [runtime launch](../../docs/development/dotnet-runtime-launch.md).

Reserved for a separately launched C#/.NET production worker using the
documented Hancom OLE/COM API. It receives validated IR, a template profile, the
runtime template path, and the output path. It then inspects and binds the
actual template, performs Hancom-specific lowering, edits a template copy, and
verifies the result. Fixture coordinates and investigation scripts do not
belong here.

The repository pins a portable .NET 10.0.400 Windows x64 SDK. The investigation
preview under `tools/investigation/hancom-automation/ir-preview` targets
`net10.0-windows`, but it does not decide the production solution/project
shape, COM interop strategy, or internal invocation protocol. Once decided,
each production or test project owns its `.csproj` and C# source files and
generated `bin/` and `obj/` output remains ignored. No exploratory program is
promoted into this directory merely by moving it.

The backend must register the official file-access security module immediately
after creating `HWPFrame.HwpObject`, process one document at a time in an
interactive Windows session, preserve the source template format, and clean up
COM state after success or failure.

All local COM investigation and backend verification must use the verified
Windows identity/profile, interactive-session contract, and host environment
recorded in `docs/development/environment.md`. Do not infer readiness from a
different account's HKCU or from a sandbox/noninteractive process.

The Hancom version in `dependencies/lock.json` is a development reference.
Security-module setup is user-managed through the official download guide;
there is no runtime hash pin or automatic installer. Tagged rendering does not
read the lock file. This directory contains
md2hwp's backend integration; exploratory C# code remains under `tools/`.
