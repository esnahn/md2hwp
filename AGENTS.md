# AGENTS.md

## v0.1.0 repository boundary

The user requested removal of tracked files not needed to build the v0.1.0
application. Historical investigations and documentation are recoverable from
Git history; do not recreate scaffolding or obsolete investigation workflows.
Keep README.md accurate. Build via tools/development/build.ps1; verify Rust with
cargo test --workspace. The old standalone smoke scripts and canonical COM probe
were removed by this cleanup; do not claim they still exist or have passed.

Rust application: apps/md2hwp. Shared semantic core: crates/md2hwp-core.
C# backend: backends/hancom-automation/Md2Hwp.Backend.csproj.
Template: templates/template.hwp. Build outputs: target/, bin/, obj/ (ignored).
The JSON fixtures still tracked are included directly by Rust test compilation.

## Contracts

Keep Pandoc invocation in the app and Pandoc AST handling in the core. Backends
consume validated IR. Preserve Unicode. Reject unsupported constructors explicitly.
Use only the current IR 0.2 closed schema. AST2IR rules target that IR without
an independent rules version. Input IR and template ir-version must match the
program's current IR exactly; reject old versions with regeneration guidance.
Template-owned begin:template/end:template declarations supply all current IR
styles and prototype ranges, including roles unused by the manuscript;
do not restore runtime external profiles. Preserve unrelated template content.
Generated links render as formatted labels/plain text; strip automatic hyperlinks
only in generated content. Figures embed PNGs; caption and source follow the figure.
Verbatim blocks may carry sources. Generated tables, HWPX and RST input are deferred.

## Dependencies

Pinned Rust and .NET SDK settings remain authoritative. Use the repository-local
.NET SDK through tools/development/dotnet.ps1. dependencies/lock.json is build/dev
metadata, not a runtime version gate. Keep the Hancom module development pin.
Never install/register the Hancom security module or mutate its DLL/HKCU settings.
Users manage it via https://developer.hancom.com/hwpautomation.
Require RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample") to return
true before opening a document. Existing file/registration checks remain mandatory.
Pandoc setup is explicit, uses the build-time preferred release or official stable
fallback, checks the digest, and preserves upstream notices. No runtime lock sidecar.

## Hancom safety and verification

Use documented COM only, not UI clicks/keystrokes. Process one document at a time.
Never modify source templates or manuscripts. Replace generated HWP only after
successful temporary rendering, save/reopen and structural verification. Preserve
the old result on failure. Do not accept security, repair or data-loss dialogs.
Do not terminate user-owned HWP processes. Require existing HWP processes to close.

Verified workstation context from the removed environment record:
DESKTOP-BRTN48S\MOLIT, interactive Session 1, Windows PowerShell 5.1 x64 STA,
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe,
Hancom Office 2020 HWP 11.0.0.9136, HWPFrame.HwpObject.
Confirm actual identity/session/host before COM work in an approved non-default
execution context. Sandbox results do not establish workstation registration.
Hidden windows in that interactive session are supported; services and concurrent
COM are not. Report when no live or visual test was performed.
For paragraph deletion select paragraph beginning through next paragraph beginning
(MoveSelNextParaBegin), then verify text. Do not use SetPos+SelectPara+Delete.
Keep native coordinates inside the adapter; no fixture coordinates in public IR.

Current shorthand: md2hwp source.md [[--output] <source.output.hwp>] creates source.ir.json and by
default source.output.hwp; backend takes source.ir.json [[--output] <source.output.hwp>]. Both
accept --template; Rust also accepts --worker and --dotnet. Option order is free.
Explicit option paths resolve from caller cwd, independently of image resources. Existing
option-based render modes remain. Defaults resolve beside the relevant EXE.
Shorthand resources resolve within the source/IR directory; explicit render modes
use cwd. Maintain source/template protection and successful-result replacement.
