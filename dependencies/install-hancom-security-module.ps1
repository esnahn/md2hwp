# Explicit dependency setup; never invoke this as a build side effect.
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw "Windows is required." }

$moduleName = "FilePathCheckerModuleExample"
$moduleFileName = "$moduleName.dll"
$lockPath = Join-Path $PSScriptRoot "lock.json"
$installedModulePath = Join-Path $PSScriptRoot $moduleFileName
$registryPath = "HKCU:\Software\HNC\HwpAutomation\Modules"
$registrySubKey = "Software\HNC\HwpAutomation\Modules"

$dependencyLock = Get-Content -Raw -Encoding UTF8 -LiteralPath $lockPath | ConvertFrom-Json
if ($dependencyLock.schema -ne "md2hwp.dependencies-lock" -or
    $dependencyLock.lock_version -ne "0.1") {
    throw "Unsupported dependency lock: $lockPath"
}

$hancomDependencies = @($dependencyLock.dependencies | Where-Object name -eq "hancom-automation")
if ($hancomDependencies.Count -ne 1) { throw "Expected one hancom-automation dependency." }

$securityPins = @($hancomDependencies[0].pins | Where-Object name -eq "file-path-checker-module-example")
if ($securityPins.Count -ne 1 -or $securityPins[0].kind -ne "content") {
    throw "Expected one file-path-checker-module-example content pin."
}

$downloadUri = [uri][string]$securityPins[0].url
$expectedSha256 = [string]$securityPins[0].sha256
if (-not $downloadUri.IsAbsoluteUri -or $downloadUri.Scheme -ne "https" -or
    $expectedSha256 -cnotmatch "^[0-9A-F]{64}$") {
    throw "The Hancom security-module URL or SHA-256 pin is invalid."
}

# Both the existing-registration and fallback paths must perform this check.
function Test-HancomModuleRegistration([string]$Name) {
    $preexistingHwp = @(Get-Process -Name "Hwp" -ErrorAction SilentlyContinue)
    if ($preexistingHwp.Count -ne 0) {
        throw "Close existing HWP processes before security-module verification: $($preexistingHwp.Id -join ', ')"
    }

    $hwp = $null
    $registered = $false
    $primaryFailure = $null
    $cleanupFailures = @()
    try {
        $hwpType = [Type]::GetTypeFromProgID("HWPFrame.HwpObject")
        if ($null -eq $hwpType) { throw "HWPFrame.HwpObject is not registered." }
        $hwp = [Activator]::CreateInstance($hwpType)
        # RegisterModule must be the first COM call after object creation.
        $registered = [bool]$hwp.RegisterModule("FilePathCheckDLL", $Name)
    }
    catch { $primaryFailure = $_ }
    finally {
        $spawnedHwp = @(Get-Process -Name "Hwp" -ErrorAction SilentlyContinue)
        if ($null -ne $hwp -and $spawnedHwp.Count -ne 1) {
            $cleanupFailures += "Expected one spawned HWP process; found $($spawnedHwp.Count)."
        }
        if ($null -ne $hwp -and
            [Runtime.InteropServices.Marshal]::IsComObject($hwp)) {
            try { $null = $hwp.Quit() }
            catch { $cleanupFailures += "Quit: $($_.Exception.Message)" }
            try { $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($hwp) }
            catch { $cleanupFailures += "COM release: $($_.Exception.Message)" }
        }
        [GC]::Collect()
        [GC]::WaitForPendingFinalizers()

        if ($spawnedHwp.Count -eq 1) {
            $ownedPid = $spawnedHwp[0].Id
            $ownedStartTime = $spawnedHwp[0].StartTime
            Wait-Process -Id $ownedPid -Timeout 10 -ErrorAction SilentlyContinue
            $remainingHwp = Get-Process -Id $ownedPid -ErrorAction SilentlyContinue
            try {
                if ($null -ne $remainingHwp -and
                    $remainingHwp.ProcessName -eq "Hwp" -and
                    $remainingHwp.StartTime -eq $ownedStartTime -and
                    $remainingHwp.MainWindowHandle -eq 0) {
                    Stop-Process -Id $ownedPid -Force
                    Wait-Process -Id $ownedPid -Timeout 5 -ErrorAction SilentlyContinue
                }
            }
            catch { $cleanupFailures += "process cleanup: $($_.Exception.Message)" }
            if ($null -ne (Get-Process -Id $ownedPid -ErrorAction SilentlyContinue)) {
                $cleanupFailures += "The owned HWP process did not exit."
            }
        }
    }

    if ($null -ne $primaryFailure) {
        if ($cleanupFailures.Count) { throw "$($primaryFailure.Exception.Message) Cleanup also failed: $($cleanupFailures -join '; ')" }
        throw $primaryFailure
    }
    if ($cleanupFailures.Count) { throw "Hancom registration cleanup failed: $($cleanupFailures -join '; ')" }
    return $registered
}

# Snapshot the named registry value before deciding whether to mutate anything.
$registryKeyExisted = $false
$previousRegistryValueExisted = $false
$previousRegistryValue = $null
$previousRegistryValueKind = $null
$registryKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($registrySubKey, $false)
try {
    if ($null -ne $registryKey) {
        $registryKeyExisted = $true
        if ($registryKey.GetValueNames() -contains $moduleName) {
            $previousRegistryValueExisted = $true
            $previousRegistryValue = $registryKey.GetValue($moduleName, $null,
                [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            $previousRegistryValueKind = $registryKey.GetValueKind($moduleName)
        }
    }
}
finally { if ($null -ne $registryKey) { $registryKey.Dispose() } }

$currentModulePath = $null
$currentModuleIsPinned = $false
if ($previousRegistryValueKind -eq [Microsoft.Win32.RegistryValueKind]::String -and
    -not [string]::IsNullOrWhiteSpace([string]$previousRegistryValue) -and
    [IO.Path]::IsPathRooted([string]$previousRegistryValue)) {
    try {
        $currentModulePath = [IO.Path]::GetFullPath([string]$previousRegistryValue)
    }
    catch [ArgumentException] {}
    catch [NotSupportedException] {}
    catch [IO.PathTooLongException] {}

    if ($null -ne $currentModulePath -and (Test-Path -LiteralPath $currentModulePath -PathType Leaf)) {
        $currentModuleIsPinned = (Get-FileHash -LiteralPath $currentModulePath -Algorithm SHA256).Hash -eq $expectedSha256
    }
}

if ($currentModuleIsPinned) {
    if (-not (Test-HancomModuleRegistration $moduleName)) {
        throw "The pinned registered module exists, but Hancom rejected it. No installation state was changed."
    }
    [pscustomobject]@{
        Action = "AlreadyValid"
        ModulePath = $currentModulePath
        ModuleSha256 = $expectedSha256
        RegisterModule = $true
    }
    return
}

$temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$temporaryLeaf = "md2hwp-security-" + [guid]::NewGuid().ToString("N")
$temporaryDirectory = [IO.Path]::GetFullPath((Join-Path $temporaryRoot $temporaryLeaf))
if (-not $temporaryDirectory.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $temporaryLeaf -notmatch '^md2hwp-security-[0-9a-f]{32}$') {
    throw "Unsafe temporary directory: $temporaryDirectory"
}

$archivePath = Join-Path $temporaryDirectory "security-module.zip"
$extractDirectory = Join-Path $temporaryDirectory "extracted"
$backupPath = Join-Path $temporaryDirectory "previous-module.dll"
$installedModuleExisted = Test-Path -LiteralPath $installedModulePath -PathType Leaf
$previousInstalledHash = $null
if ($installedModuleExisted) {
    $previousInstalledHash = (Get-FileHash -LiteralPath $installedModulePath -Algorithm SHA256).Hash
}
$mutationStarted = $false
$action = $null

try {
    $null = New-Item -ItemType Directory -Path $extractDirectory
    Invoke-WebRequest -UseBasicParsing -Uri $downloadUri -OutFile $archivePath
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDirectory

    $moduleFiles = @(Get-ChildItem -LiteralPath $extractDirectory -Filter $moduleFileName -File -Recurse)
    if ($moduleFiles.Count -ne 1) {
        throw "Expected exactly one $moduleFileName in the official archive; found $($moduleFiles.Count)."
    }
    if ((Get-FileHash -LiteralPath $moduleFiles[0].FullName -Algorithm SHA256).Hash -ne $expectedSha256) {
        throw "The downloaded Hancom security-module hash does not match lock.json."
    }

    if ($installedModuleExisted) {
        Copy-Item -LiteralPath $installedModulePath -Destination $backupPath
        if ((Get-FileHash -LiteralPath $backupPath -Algorithm SHA256).Hash -ne $previousInstalledHash) {
            throw "Could not verify the previous managed DLL backup."
        }
    }

    $mutationStarted = $true
    Copy-Item -LiteralPath $moduleFiles[0].FullName -Destination $installedModulePath -Force
    if ((Get-FileHash -LiteralPath $installedModulePath -Algorithm SHA256).Hash -ne $expectedSha256) {
        throw "The installed Hancom security-module hash does not match lock.json."
    }

    $registryKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($registrySubKey)
    try {
        $registryKey.SetValue($moduleName, $installedModulePath,
            [Microsoft.Win32.RegistryValueKind]::String)
        if ($registryKey.GetValueKind($moduleName) -ne [Microsoft.Win32.RegistryValueKind]::String -or
            [string]$registryKey.GetValue($moduleName) -cne $installedModulePath) {
            throw "Could not verify the Hancom security-module registry value."
        }
    }
    finally { $registryKey.Dispose() }

    if (-not (Test-HancomModuleRegistration $moduleName)) {
        throw "Hancom rejected the registered file-access security module."
    }
    $action = if ($previousRegistryValueExisted) { "Repaired" } else { "Installed" }
}
catch {
    $primaryFailure = $_
    $rollbackFailures = @()

    if ($mutationStarted) {
        try {
            if ($previousRegistryValueExisted) {
                $registryKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($registrySubKey)
                try {
                    $registryKey.SetValue($moduleName, $previousRegistryValue,
                        $previousRegistryValueKind)
                }
                finally { $registryKey.Dispose() }
            }
            else {
                $registryKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($registrySubKey, $true)
                try {
                    if ($null -ne $registryKey -and
                        $registryKey.GetValueNames() -contains $moduleName) {
                        $registryKey.DeleteValue($moduleName, $false)
                    }
                }
                finally { if ($null -ne $registryKey) { $registryKey.Dispose() } }

                if (-not $registryKeyExisted -and (Test-Path -LiteralPath $registryPath)) {
                    $registryKey = Get-Item -LiteralPath $registryPath
                    if ($registryKey.ValueCount -eq 0 -and $registryKey.SubKeyCount -eq 0) {
                        Remove-Item -LiteralPath $registryPath
                    }
                }
            }
        }
        catch {
            $rollbackFailures += "registry: $($_.Exception.Message)"
        }

        try {
            if ($installedModuleExisted) {
                Copy-Item -LiteralPath $backupPath -Destination $installedModulePath -Force
                if ((Get-FileHash -LiteralPath $installedModulePath -Algorithm SHA256).Hash -ne $previousInstalledHash) {
                    throw "The restored managed DLL hash does not match its backup."
                }
            }
            elseif (Test-Path -LiteralPath $installedModulePath -PathType Leaf) {
                Remove-Item -LiteralPath $installedModulePath -Force
            }
        }
        catch {
            $rollbackFailures += "file: $($_.Exception.Message)"
        }
    }

    if ($rollbackFailures.Count -ne 0) {
        throw "Installation failed: $($primaryFailure.Exception.Message) Rollback also failed: $($rollbackFailures -join '; ')"
    }
    throw $primaryFailure
}
finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        if (-not $temporaryDirectory.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase) -or
            $temporaryLeaf -notmatch '^md2hwp-security-[0-9a-f]{32}$') {
            Write-Warning "Refusing to remove unsafe temporary path: $temporaryDirectory"
        }
        else {
            try { Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force }
            catch { Write-Warning "Could not remove temporary installer directory: $($_.Exception.Message)" }
        }
    }
}

[pscustomobject]@{
    Action = $action
    ModulePath = $installedModulePath
    ModuleSha256 = $expectedSha256
    RegisterModule = $true
}
