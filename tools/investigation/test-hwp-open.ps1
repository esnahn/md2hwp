[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$File,

    [string]$ProgId = "HWPFrame.HwpObject",

    [string]$ModuleName = "FilePathCheckerModuleExample",

    [ValidateRange(1, 10)]
    [int]$RegisterAttempts = 1,

    [ValidateRange(0, 5000)]
    [int]$RegisterRetryMilliseconds = 1000,

    [switch]$Visible
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$requiredHostProcess = 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
$requiredInvocation = "$requiredHostProcess -NoProfile -ExecutionPolicy Bypass -Sta -File .\tools\investigation\test-hwp-open.ps1 -File <path-to-hwp>"
$actualHostProcess = [IO.Path]::GetFullPath(
    [Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
)
if ($PSVersionTable.PSEdition -ne "Desktop" -or
    $PSVersionTable.PSVersion.Major -ne 5 -or
    $PSVersionTable.PSVersion.Minor -ne 1 -or
    -not [Environment]::Is64BitProcess -or
    [Threading.Thread]::CurrentThread.ApartmentState -ne [Threading.ApartmentState]::STA -or
    -not [Environment]::UserInteractive -or
    [Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0 -or
    -not $actualHostProcess.Equals(
        $requiredHostProcess,
        [StringComparison]::OrdinalIgnoreCase
    )) {
    throw "Run Hancom COM probes in the verified interactive Windows PowerShell 5.1 x64 STA environment: $requiredInvocation"
}

$absoluteFile = [IO.Path]::GetFullPath($File)
if (-not (Test-Path -LiteralPath $absoluteFile -PathType Leaf)) {
    throw "Missing HWP test file: $absoluteFile"
}

$moduleRegistryPath = "HKCU:\Software\HNC\HwpAutomation\Modules"
$modulePath = $null
$moduleRegistryValueKind = $null
if (Test-Path -LiteralPath $moduleRegistryPath) {
    $moduleRegistryKey = Get-Item -LiteralPath $moduleRegistryPath
    if ($moduleRegistryKey.GetValueNames() -contains $ModuleName) {
        $modulePath = [string]$moduleRegistryKey.GetValue(
            $ModuleName,
            $null,
            [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
        )
        $moduleRegistryValueKind = $moduleRegistryKey.GetValueKind($ModuleName)
    }
}
$dependencyLockPath = [IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot "..\..\dependencies\lock.json")
)
$dependencyLock = Get-Content -Raw -Encoding UTF8 -LiteralPath $dependencyLockPath |
    ConvertFrom-Json
$securityPins = @(
    $dependencyLock.dependencies |
        Where-Object { $_.name -eq "hancom-automation" } |
        Select-Object -ExpandProperty pins |
        Where-Object { $_.name -eq "file-path-checker-module-example" }
)
if ($securityPins.Count -ne 1) {
    throw "Expected exactly one pinned Hancom file-path checker module."
}
$expectedModuleSha256 = [string]$securityPins[0].sha256
if ([string]::IsNullOrWhiteSpace($modulePath) -or
    -not (Test-Path -LiteralPath $modulePath -PathType Leaf)) {
    throw "The registered Hancom security module is missing: $modulePath"
}
if ($moduleRegistryValueKind -ne [Microsoft.Win32.RegistryValueKind]::String) {
    throw "The Hancom security-module registry value must be REG_SZ. Found: $moduleRegistryValueKind"
}
if (-not [IO.Path]::IsPathRooted($modulePath)) {
    throw "The Hancom security-module registry value must be an absolute path: $modulePath"
}
$modulePath = [IO.Path]::GetFullPath($modulePath)
$moduleSha256 = (Get-FileHash -LiteralPath $modulePath -Algorithm SHA256).Hash
if ($moduleSha256 -ne $expectedModuleSha256) {
    throw "The registered Hancom security module hash does not match dependencies/lock.json."
}

$preexistingHwpIds = @(
    Get-Process -Name "Hwp" -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Id
)
if ($preexistingHwpIds.Count -ne 0) {
    throw "Close existing HWP processes before running the isolated open probe: $($preexistingHwpIds -join ', ')"
}
$hwp = $null
$hwpPid = $null
$hwpStartTime = $null
$hwpProcessExited = $null
$hwpForcedTermination = $false
$registered = $false
$registerAttempt = 0
$opened = $false
$loadedModulePath = $null
$loadedModuleMatchesRegistry = $null
$loadedModuleCandidates = @()
$moduleLoadObservation = "NotAttempted"
$failure = $null
$exitCode = 0

try {
    $hwp = New-Object -ComObject $ProgId
    # RegisterModule is intentionally the first COM call after creation.
    for ($attempt = 1; $attempt -le $RegisterAttempts; $attempt++) {
        $registerAttempt = $attempt
        $registered = [bool]$hwp.RegisterModule("FilePathCheckDLL", $ModuleName)
        if ($registered) {
            break
        }
        if ($attempt -lt $RegisterAttempts -and $RegisterRetryMilliseconds -gt 0) {
            Start-Sleep -Milliseconds $RegisterRetryMilliseconds
        }
    }

    $processDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $newHwpProcesses = @(
            Get-Process -Name "Hwp" -ErrorAction SilentlyContinue |
                Where-Object { $_.Id -notin $preexistingHwpIds } |
                Sort-Object StartTime
        )
        if ($newHwpProcesses.Count -gt 1) {
            throw "Expected one isolated HWP process; found $($newHwpProcesses.Count)."
        }
        if ($newHwpProcesses.Count -eq 1) {
            $hwpPid = $newHwpProcesses[0].Id
            $hwpStartTime = $newHwpProcesses[0].StartTime
            break
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $processDeadline)

    if (-not $registered) {
        $exitCode = 2
    }
    else {
        if ($null -eq $hwpPid) {
            $failure = "Could not identify the isolated HWP process."
            $exitCode = 5
        }
        else {
            if ($Visible) {
                $hwp.XHwpWindows.Item(0).Visible = $true
            }
            $opened = [bool]$hwp.Open(
                $absoluteFile,
                "HWP",
                "lock:false;forceopen:true;suspendpassword:true;versionwarning:false"
            )
            if (-not $opened) {
                $exitCode = 3
            }
            else {
                # Process.Modules is diagnostic only. Hancom's supported
                # success contract is RegisterModule=true followed by
                # Open=true without an approval prompt. HWP does not document
                # that the checker remains loaded after Open returns, and an
                # x64 .NET caller can omit x86 modules from this snapshot.
                $moduleLoadObservation = "NotObservedInProcessSnapshot"
                try {
                    $hwpProcess = Get-Process -Id $hwpPid -ErrorAction Stop
                    $allLoadedModulePaths = @(
                        $hwpProcess.Modules |
                            Select-Object -ExpandProperty FileName
                    )
                    $loadedModulePaths = @(
                        $allLoadedModulePaths |
                            Where-Object { $_ -ieq $modulePath }
                    )
                    $loadedModuleCandidates = @(
                        $allLoadedModulePaths |
                            Where-Object {
                                $_ -match '(?i)(FilePathCheckerModuleExample|chkfilepath)\.dll$'
                            }
                    )

                    if ($loadedModulePaths.Count -eq 1) {
                        $loadedModulePath = [string]$loadedModulePaths[0]
                        $loadedModuleMatchesRegistry = $true
                        $moduleLoadObservation = "ObservedAfterOpen"
                    }
                    elseif ($loadedModulePaths.Count -gt 1) {
                        $moduleLoadObservation = "MultipleMatchesObservedAfterOpen"
                    }
                    elseif ($loadedModuleCandidates.Count -gt 0) {
                        $loadedModuleMatchesRegistry = $false
                        $moduleLoadObservation = "DifferentCheckerObservedAfterOpen"
                    }
                }
                catch {
                    $moduleLoadObservation = "EnumerationUnavailable"
                }
            }
        }
    }
}
catch {
    $failure = $_.Exception.ToString()
    $exitCode = 1
}
finally {
    if ($null -ne $hwp) {
        if ($opened) {
            try { $null = $hwp.Clear(1) } catch {}
        }
        try { $null = $hwp.Quit() } catch {}
        if ([Runtime.InteropServices.Marshal]::IsComObject($hwp)) {
            try {
                $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($hwp)
            }
            catch {
                if ($null -eq $failure) {
                    $failure = "COM release failed: $($_.Exception.Message)"
                }
                else {
                    $failure = "$failure Cleanup also failed: $($_.Exception.Message)"
                }
                $exitCode = 4
            }
        }
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
    if ($null -eq $hwpPid) {
        $untrackedHwpProcesses = @(
            Get-Process -Name "Hwp" -ErrorAction SilentlyContinue |
                Where-Object { $_.Id -notin $preexistingHwpIds }
        )
        if ($untrackedHwpProcesses.Count -eq 1) {
            $hwpPid = $untrackedHwpProcesses[0].Id
            $hwpStartTime = $untrackedHwpProcesses[0].StartTime
        }
        elseif ($untrackedHwpProcesses.Count -gt 1) {
            $failure = "Could not uniquely identify $($untrackedHwpProcesses.Count) spawned HWP processes during cleanup."
            $exitCode = 4
        }
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
    if ($null -ne $hwpPid) {
        Wait-Process -Id $hwpPid -Timeout 10 -ErrorAction SilentlyContinue
        $remainingHwp = Get-Process -Id $hwpPid -ErrorAction SilentlyContinue
        if ($null -ne $remainingHwp) {
            if ($remainingHwp.ProcessName -eq "Hwp" -and
                $remainingHwp.StartTime -eq $hwpStartTime -and
                $remainingHwp.MainWindowHandle -eq 0) {
                Stop-Process -Id $hwpPid -Force
                $hwpForcedTermination = $true
                Wait-Process -Id $hwpPid -Timeout 5 -ErrorAction SilentlyContinue
            }
            else {
                $failure = "The spawned HWP PID no longer matches the isolated automation process."
                $exitCode = 4
            }
        }
        $hwpProcessExited = $null -eq (
            Get-Process -Id $hwpPid -ErrorAction SilentlyContinue
        )
        if (-not $hwpProcessExited) {
            $failure = "The isolated HWP process did not exit after cleanup."
            $exitCode = 4
        }
    }
}

[pscustomobject]@{
    User = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    SessionId = [Diagnostics.Process]::GetCurrentProcess().SessionId
    PowerShell = $PSVersionTable.PSVersion.ToString()
    PSEdition = $PSVersionTable.PSEdition
    Is64BitProcess = [Environment]::Is64BitProcess
    ApartmentState = [Threading.Thread]::CurrentThread.ApartmentState
    HostProcess = [Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
    ProgId = $ProgId
    Visible = [bool]$Visible
    ModuleName = $ModuleName
    ModuleRegistryView = "Verified Windows PowerShell HKCU provider"
    ModuleRegistryValueKind = $moduleRegistryValueKind
    ModulePath = $modulePath
    ModuleFileExists = ($null -ne $modulePath -and
        (Test-Path -LiteralPath $modulePath -PathType Leaf))
    ModuleSha256 = $moduleSha256
    LoadedModulePath = $loadedModulePath
    LoadedModuleMatchesRegistry = $loadedModuleMatchesRegistry
    LoadedModuleCandidates = $loadedModuleCandidates
    ModuleLoadObservation = $moduleLoadObservation
    HwpPid = $hwpPid
    HwpProcessExited = $hwpProcessExited
    HwpForcedTermination = $hwpForcedTermination
    RegisterModule = $registered
    RegisterAttempt = $registerAttempt
    Open = $opened
    File = $absoluteFile
    Failure = $failure
    ExitCode = $exitCode
}

exit $exitCode
