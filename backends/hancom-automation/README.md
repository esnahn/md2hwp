# Hancom Automation backend

Reserved for a separately launched C#/.NET production worker using the
documented Hancom OLE/COM API. It receives validated IR, a template profile, the
runtime template path, and the output path. It then inspects and binds the
actual template, performs Hancom-specific lowering, edits a template copy, and
verifies the result. Fixture coordinates and investigation scripts do not
belong here.

The .NET SDK, target framework, solution/project files, COM interop strategy,
and internal invocation protocol remain open decisions. Once decided, each
production or test project owns its `.csproj` and C# source files and generated
`bin/` and `obj/` output remains ignored. No empty project is scaffolded merely
to make this boundary look implemented, and no exploratory PowerShell script
should be promoted into this directory merely by moving it.

The backend must register the official file-access security module immediately
after creating `HWPFrame.HwpObject`, process one document at a time in an
interactive Windows session, preserve the source template format, and clean up
COM state after success or failure.

All local COM investigation and backend verification must use the verified
Windows identity/profile, interactive-session contract, and host environment
recorded in `docs/development/environment.md`. Do not infer readiness from a
different account's HKCU or from a sandbox/noninteractive process.

Hancom Office/runtime prerequisites and the security-module pin live in
`dependencies/lock.json`; the explicit installer is
`dependencies/install-hancom-security-module.ps1`. This directory contains
only md2hwp's C# backend implementation and its tests.
