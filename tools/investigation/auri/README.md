# AURI investigations

Scripts here record behavior observed against specific AURI fixtures and the
reference Hancom environment. Hard-coded paragraph/style coordinates are
allowed only when clearly labeled as fixture observations. They are not a
template insertion contract or a production backend.

Run the existing format example builder from the repository root so its
relative default input paths resolve as documented.

`inspect-template-structure.ps1` is a read-only HWPML structure inventory. It
requires the verified interactive Windows PowerShell 5.1 x64 STA context,
validates the registered file-access module against `dependencies/lock.json`,
refuses to run beside an existing HWP process, and checks the source hash again
after cleanup. It emits JSON to standard output and does not write an artifact.

From the repository root, inspect the complete template or request concise
details for selected root paragraphs:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\auri\inspect-template-structure.ps1 `
  -File '.\reference\auri\01 auri 기본연구보고서 작성양식.hwp'

C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\auri\inspect-template-structure.ps1 `
  -File '.\reference\auri\01 auri 기본연구보고서 작성양식.hwp' `
  -RootIndexes '294,314'
```

Root indexes are reported observations for one exact file identity. They are
diagnostic addresses, not production profile selectors or converter inputs.
