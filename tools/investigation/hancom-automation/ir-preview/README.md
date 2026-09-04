# C# Hancom IR preview investigation

This is an investigation program, not the production Hancom Automation
backend. It shows how validated IR can drive a safe edit of an HWP copy before
the template-profile and lowering contracts are implemented.

It has four modes:

- `plan` parses the closed IR v0.1 shape without COM and emits the exact preview
  operations as JSON;
- `probe` registers the security module, opens an HWP without saving, closes
  it, and verifies that the input hash did not change;
- `render` copies an HWP to a temporary sibling, appends diagnostic text and
  repository-local PNG figures from IR, saves and reopens it, verifies a text
  marker and picture count, then publishes the requested output path.
- `export-images` opens an HWP read-only, uses Hancom's PNG `SaveAs` support to
  render every page into a new output directory, validates every PNG header and
  records its dimensions and hash, and verifies that the HWP did not change.

The whole-document PNG call follows Hancom's documented Automation example:
[`SaveAs(path, "PNG", "")`](https://forum.developer.hancom.com/t/createpageimage/1861).

The preview deliberately flattens symbolic styles, character marks, links, and
native list semantics into labeled diagnostic paragraphs. It also previews IR
line breaks as separate paragraphs. These limitations are present in `plan`
output and must not be copied into production lowering.

## Setup and non-COM verification

From PowerShell 7:

```powershell
pwsh -NoProfile -File .\dependencies\install-dotnet-sdk.ps1
pwsh -NoProfile -File .\tools\development\dotnet.ps1 build `
  .\tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj `
  --configuration Release
pwsh -NoProfile -File .\tools\smoke\test-hancom-ir-preview-plan.ps1
```

The SDK is the locked repository-local .NET 10.0.400 Windows x64 SDK. The
project uses no external NuGet package and its restore configuration does not
read user-level NuGet settings.

## Manual COM probe and render

Do not run COM modes from the default Codex sandbox, PowerShell 7, a service,
or a noninteractive session. Use the verified Windows PowerShell 5.1 x64 STA
context and run the open-only mode first in a fresh process:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode probe `
  -Template .\tests\fixtures\templates\minimal.hwp
```

The C# child process on the reference workstation was adopted after this probe
and a complete diagnostic render passed on 2026-08-28. A different workstation
or Windows identity must establish its own result in
`docs/development/environment.md` before relying on `render`. The diagnostic
edit command is:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode render `
  -Ir .\examples\ir-v0.1.json `
  -Template .\tests\fixtures\templates\minimal.hwp `
  -Output .\artifacts\csharp-ir-preview.hwp `
  -Visible
```

The visible render above is for a human-run diagnostic only.
An automated invocation can omit `-Visible` and keep HWP hidden.
The source template is never opened for writing. Existing output is rejected,
and failed rendering leaves neither the requested output nor a temporary copy.
HWP 2020 returned a COM object from `InsertPicture` in the adopted .NET
late-binding context; the preview accepts that result but still requires the
saved-and-reopened picture count to increase by the exact expected amount.

For visual review, export all pages from a completed preview into a directory
that does not already exist:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode export-images `
  -Document .\artifacts\csharp-ir-preview.hwp `
  -Output .\artifacts\csharp-ir-preview-pages
```

The first reference-workstation export was checked on 2026-08-29. Hancom 2020
produced `page001.png` through `page003.png`, each 992 by 1403 pixels, and left
the HWP unchanged. The images confirmed that the template cover and page
decorations survived and that the 142 mm figure retained its 3:2 ratio. They
also made the preview's non-production behavior visible: diagnostic content
starts at the document end, the figure label can be separated from the image by
pagination, symbolic formatting remains flattened, and the template's current
font renders the city emoji as missing-glyph boxes. A separate save/reopen
probe confirmed that this is a rendering limitation in the tested path, not
loss of the underlying `U+1F3D9 U+FE0F` values. Generated page images stay under
ignored `artifacts/`; they are evidence for human review, not fixtures.
