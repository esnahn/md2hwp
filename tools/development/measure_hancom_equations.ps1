# SPDX-License-Identifier: AGPL-3.0-only
<#
.SYNOPSIS
Measure native Hancom equation geometry for explicit (script, base unit) cases.

.DESCRIPTION
Run in Windows PowerShell 5.1 x64 STA in the interactive session. Requires the
registered FilePathCheckerModuleExample module. Hancom windows already open are
recorded and never touched: the script starts and closes only its own hidden HWP
process, creates each equation with EquationCreate
(default HancomEQN font) in a disposable document and records the exported
EQUATION width, height and baseline. It never opens or saves user documents.

Input JSON: {"cases":[{"script":"a + b","base_unit":1000}, ...]}
Output JSON: {"hancom_version":..., "cases":[{"script","base_unit","width","height","baseline"}]}
#>
param(
    [Parameter(Mandatory)][string]$InputPath,
    [Parameter(Mandatory)][string]$OutputPath,
    [int]$BatchSize = 150
)
$ErrorActionPreference = 'Stop'
if ([Threading.Thread]::CurrentThread.ApartmentState -ne 'STA' -or -not [Environment]::Is64BitProcess) {
    throw 'Run in 64-bit Windows PowerShell with -STA.'
}
$source = [IO.File]::ReadAllText((Resolve-Path $InputPath), [Text.Encoding]::UTF8) | ConvertFrom-Json
$cases = @($source.cases)
if ($cases.Count -eq 0) { throw 'Input has no cases.' }
foreach ($case in $cases) {
    if ([string]::IsNullOrWhiteSpace([string]$case.script) -or [int]$case.base_unit -le 0) {
        throw 'Every case requires a nonblank script and a positive base_unit.'
    }
}

if (-not ('Md2HwpMeasure.NativeWindow' -as [type])) {
    Add-Type -Namespace Md2HwpMeasure -Name NativeWindow -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true)]
public static extern uint GetWindowThreadProcessId(System.IntPtr hWnd, out uint processId);
'@
}
$before = @{}
foreach ($p in @(Get-Process hwp,hwp64 -ErrorAction SilentlyContinue)) { $before[[int]$p.Id] = $p.StartTime }
$hwp = New-Object -ComObject HWPFrame.HwpObject
# Identify the process behind this COM object; never touch one that already existed.
[uint32]$ownedId = 0
[void][Md2HwpMeasure.NativeWindow]::GetWindowThreadProcessId([IntPtr][long]$hwp.XHwpWindows.Active_XHwpWindow.WindowHandle, [ref]$ownedId)
if ($ownedId -eq 0 -or $before.ContainsKey([int]$ownedId)) {
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($hwp)
    throw "COM did not start a new Hancom process ($ownedId); it was left untouched."
}
$ownedStart = (Get-Process -Id $ownedId).StartTime
$results = New-Object System.Collections.Generic.List[object]
try {
    if (-not $hwp.RegisterModule('FilePathCheckDLL', 'FilePathCheckerModuleExample')) { throw 'RegisterModule failed.' }
    $hwp.XHwpWindows.Active_XHwpWindow.Visible = $false
    for ($start = 0; $start -lt $cases.Count; $start += $BatchSize) {
        $batch = @($cases[$start..([Math]::Min($start + $BatchSize, $cases.Count) - 1)])
        $hwp.Clear(1)
        foreach ($case in $batch) {
            # One parameter object keeps GetDefault values (inline, margins, version).
            $p = $hwp.HParameterSet.HEqEdit
            [void]$hwp.HAction.GetDefault('EquationCreate', $p.HSet)
            $p.string = [string]$case.script
            $p.BaseUnit = [int]$case.base_unit
            if (-not $hwp.HAction.Execute('EquationCreate', $p.HSet)) { throw "EquationCreate failed: $($case.script)" }
            [void]$hwp.HAction.Run('MoveDocEnd')
            [void]$hwp.HAction.Run('BreakPara')
        }
        $xml = [xml]$hwp.GetTextFile('HWPML2X', '')
        $equations = @($xml.GetElementsByTagName('EQUATION'))
        if ($equations.Count -ne $batch.Count) { throw "Expected $($batch.Count) equations, found $($equations.Count)." }
        for ($i = 0; $i -lt $batch.Count; $i++) {
            $e = $equations[$i]
            $shape = $e.SHAPEOBJECT
            if ($shape.POSITION.TreatAsChar -ne 'true') { throw 'An equation was not created inline.' }
            if ($e.SCRIPT -ne [string]$batch[$i].script) { throw "Hancom changed script: $($batch[$i].script) -> $($e.SCRIPT)" }
            $results.Add([ordered]@{
                script = [string]$batch[$i].script
                base_unit = [int]$batch[$i].base_unit
                width = [int]$shape.SIZE.Width
                height = [int]$shape.SIZE.Height
                baseline = [int]$e.BaseLine
            })
        }
        Write-Host "measured $($results.Count) / $($cases.Count)"
    }
    $document = [ordered]@{
        source = 'Hancom EquationCreate, default font, tools/development/measure_hancom_equations.ps1'
        hancom_version = [string]$hwp.Version
        cases = $results
    }
    $json = $document | ConvertTo-Json -Depth 4
    [IO.File]::WriteAllText([IO.Path]::GetFullPath($OutputPath), $json + "`n", (New-Object Text.UTF8Encoding($false)))
} finally {
    try { $hwp.Clear(1); $hwp.Quit() } catch { }
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($hwp)
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        $q = Get-Process -Id $ownedId -ErrorAction SilentlyContinue
        if (-not $q -or $q.StartTime -ne $ownedStart) { break }
        Start-Sleep -Milliseconds 200
    }
    $q = Get-Process -Id $ownedId -ErrorAction SilentlyContinue
    if ($q -and $q.StartTime -eq $ownedStart) {
        # Only the process this script started may be stopped.
        Stop-Process -Id $ownedId -Force
        Write-Warning "Owned HWP process $ownedId did not exit after Quit and was stopped."
    }
}
