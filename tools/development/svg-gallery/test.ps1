#requires -Version 7.4
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Euisoon Ahn
<#
.SYNOPSIS
Converts the matplotlib example gallery with svg2emf and compares the EMFs, drawn by
Windows GDI+, with resvg renderings of the same SVGs.

.DESCRIPTION
A manual regression check for the EMF writer in crates/md2hwp-svg; builds and tests never
run it. It downloads matplotlib from PyPI and the gallery examples of the same release
from GitHub, then RUNS those examples. Nothing is written outside the work directory.
Expect about 1.6 GB there and tens of minutes. Earlier results are in docs/svg-figures.md.

Steps, each skipped when its result already exists in the work directory:
  1. Python virtual environment with the pinned requirements.txt
  2. sparse checkout of galleries/examples at the pinned matplotlib tag and commit
  3. every example run without a GUI, each open figure saved as SVG
  4. svg2emf: each SVG to EMF and to PNG (resvg)
  5. each EMF drawn with Windows GDI+
  6. pixel comparison of 5 with the PNG of 4

.PARAMETER WorkDirectory
Where downloads and results go. Default: target/svg-gallery (ignored by Git).
.PARAMETER Svg2Emf
The converter to test. Default: target/release/svg2emf.exe.
.PARAMETER Python
Python used to create the virtual environment.
.PARAMETER Fresh
Delete the generated figures and results (not the downloads) before running.
#>
[CmdletBinding()]
param(
    [string]$WorkDirectory,
    [string]$Svg2Emf,
    [string]$Python = 'python',
    [int]$Width = 800,
    [switch]$Fresh
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$tag = 'v3.11.2'
$commit = 'd3ca9172e392048ec242b90e8423e88d1a3000b2'
if (-not $WorkDirectory) { $WorkDirectory = Join-Path $root 'target/svg-gallery' }
if (-not $Svg2Emf) { $Svg2Emf = Join-Path $root 'target/release/svg2emf.exe' }
if (-not (Test-Path -LiteralPath $Svg2Emf -PathType Leaf)) {
    throw "svg2emf not found: $Svg2Emf. Build it with tools/development/build.ps1 -Configuration Release."
}
$Svg2Emf = (Resolve-Path -LiteralPath $Svg2Emf).Path
$work = [IO.Path]::GetFullPath($WorkDirectory)
$environment = Join-Path $work 'python'
$source = Join-Path $work 'matplotlib'
$out = Join-Path $work 'out'
$py = Join-Path $environment 'Scripts/python.exe'
New-Item -ItemType Directory -Force $work | Out-Null

function Invoke-Step([string]$What, [scriptblock]$Command) {
    Write-Host "== $What"
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What failed (exit code $LASTEXITCODE)." }
}

if (-not (Test-Path -LiteralPath $py)) {
    Invoke-Step 'Create the Python environment' { & $Python -m venv $environment }
    Invoke-Step 'Install matplotlib, numpy and pillow' {
        & $py -m pip install --disable-pip-version-check -r (Join-Path $PSScriptRoot 'requirements.txt')
    }
}
if (-not (Test-Path -LiteralPath (Join-Path $source 'galleries/examples'))) {
    if (Test-Path -LiteralPath $source) { throw "Incomplete checkout: delete $source and run again." }
    Invoke-Step "Fetch the matplotlib $tag gallery" {
        git clone --quiet --depth 1 --branch $tag --filter=blob:none --sparse https://github.com/matplotlib/matplotlib.git $source
    }
    Invoke-Step 'Check out galleries/examples' { git -C $source sparse-checkout set galleries/examples }
}
$head = git -C $source rev-parse HEAD
if ($head -ne $commit) { throw "The gallery checkout is at $head, expected $commit ($tag)." }

if ($Fresh -and (Test-Path -LiteralPath $out)) { Remove-Item -LiteralPath $out -Recurse -Force }
$examples = Join-Path $source 'galleries/examples'
$tools = $PSScriptRoot
if (-not (Test-Path -LiteralPath (Join-Path $out 'figures.json'))) {
    Invoke-Step 'Run the examples and save their figures as SVG' {
        & $py -I (Join-Path $tools 'gallery.py') figures $examples $out $Svg2Emf
    }
}
# Conversion and comparison always rerun: they are what a changed writer affects.
Get-ChildItem -LiteralPath $out -File |
    Where-Object { $_.Extension -in '.emf', '.png' } | Remove-Item -Force
Invoke-Step 'Convert every SVG with svg2emf' {
    & $py -I (Join-Path $tools 'gallery.py') convert $examples $out $Svg2Emf
}
Invoke-Step 'Draw every EMF with Windows GDI+' {
    & (Join-Path $tools 'render-emf.ps1') -Directory $out -Width $Width
}
Invoke-Step 'Compare with the resvg renderings' { & $py -I (Join-Path $tools 'compare.py') $out }

$converted = Get-Content -LiteralPath (Join-Path $out 'convert.json') -Raw | ConvertFrom-Json -AsHashtable
$failures = @($converted.GetEnumerator() | Where-Object Value)
Write-Host "== $($converted.Count) figures, $($converted.Count - $failures.Count) as EMF, $($failures.Count) not converted"
$failures | Group-Object { ($_.Value -replace '^svg2emf: ', '' -replace ';.*$', '') } | Sort-Object Count -Descending |
    ForEach-Object { Write-Host ("  {0,4}  {1}" -f $_.Count, $_.Name) }
Write-Host "Results: $out (figures.json, convert.json, compare.json)"