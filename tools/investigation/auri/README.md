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

For a concise row/cell inventory of selected zero-based tables, use
`-TableIndexes`:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\auri\inspect-template-structure.ps1 `
  -File '.\reference\auri\★ 연구보고서 스타일 목록표.hwp' `
  -TableIndexes '0'
```

Root and table indexes are reported observations for one exact file identity.
They are diagnostic addresses, not production profile selectors or converter
inputs.

Use a pipe-delimited `-StyleNames` value to inspect the linked native
`STYLE`, `CHARSHAPE`, and `PARASHAPE` definitions without writing the HWP:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\auri\inspect-template-structure.ps1 `
  -File .\tests\fixtures\templates\minimal.hwp `
  -StyleNames '본문|장제목 (개요 1)|표그림_캡션|박스내용'
```

This comparison is evidence, not a style-rewrite policy. For the tracked
minimal fixture, the native definitions in `minimal.hwp` are the rendering
contract. A separate style-list HWP may document different values; report that
difference instead of mutating the fixture implicitly.
