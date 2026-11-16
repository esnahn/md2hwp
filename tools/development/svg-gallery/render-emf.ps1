# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Euisoon Ahn
# Draws every EMF in a directory with Windows GDI+ on white, as <name>.gdiplus.png.
# Runs in Windows PowerShell 5.1 or PowerShell 7 on Windows.
param([Parameter(Mandatory)][string]$Directory, [int]$Width = 800)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$count = 0
$failed = 0
foreach ($file in Get-ChildItem -LiteralPath $Directory -Filter *.emf) {
    try {
        $emf = New-Object System.Drawing.Imaging.Metafile $file.FullName
        # rclFrame of EMR_HEADER (0.01 mm) gives the aspect ratio.
        $bytes = [IO.File]::ReadAllBytes($file.FullName)
        $w = [BitConverter]::ToInt32($bytes, 32) - [BitConverter]::ToInt32($bytes, 24)
        $h = [BitConverter]::ToInt32($bytes, 36) - [BitConverter]::ToInt32($bytes, 28)
        $height = [int][Math]::Round($Width * $h / $w)
        $bitmap = New-Object System.Drawing.Bitmap $Width, $height
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.Clear([System.Drawing.Color]::White)
        $graphics.DrawImage($emf, 0, 0, $Width, $height)
        $graphics.Dispose()
        $bitmap.Save(($file.FullName -replace '\.emf$', '.gdiplus.png'), [System.Drawing.Imaging.ImageFormat]::Png)
        $bitmap.Dispose(); $emf.Dispose(); $count++
    } catch { "FAILED $($file.Name): $_"; $failed++ }
}
"rendered $count, failed $failed"
if ($failed -ne 0) { exit 1 }