# SPDX-License-Identifier: AGPL-3.0-only
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Read-BuildToolsLock {
    param([string]$Path)
    $lock = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    $entries = @($lock.dependencies | Where-Object name -eq 'windows-build-tools')
    if ($entries.Count -ne 1) { throw 'Expected one windows-build-tools dependency.' }
    $result = @{}
    foreach ($name in @('visual-studio-build-tools', 'msvc-x64-x86', 'windows-sdk')) {
        $pins = @($entries[0].pins | Where-Object name -eq $name)
        if ($pins.Count -ne 1) { throw "Expected one $name pin." }
        $pin = $pins[0]
        if ($pin.kind -ne 'system-reference' -or [string]::IsNullOrWhiteSpace($pin.identifier)) {
            throw "Invalid system reference: $name."
        }
        $null = [version]$pin.version
        $result[$name] = $pin
    }
    $product = $result['visual-studio-build-tools']
    if ($product.identifier -ne 'Microsoft.VisualStudio.Product.BuildTools' -or
        $product.url -cne 'https://aka.ms/vs/17/release/vs_buildtools.exe' -or
        $product.channel_id -ne 'VisualStudio.17.Release') {
        throw 'Expected the official Visual Studio 2022 Build Tools release bootstrapper.'
    }
    foreach ($name in @('msvc-x64-x86', 'windows-sdk')) {
        if ($result[$name].identifier -notmatch '^Microsoft\.VisualStudio\.Component\.[A-Za-z0-9.]+$') {
            throw "Invalid component ID: $name."
        }
    }
    return $result
}

function Get-BuildToolsStatus {
    param([object[]]$Instances, [hashtable]$Pins, [string]$SdkRoot)
    $sdk = $null
    $libRoot = Join-Path $SdkRoot 'Lib'
    if (Test-Path -LiteralPath $libRoot) {
        $sdk = Get-ChildItem -LiteralPath $libRoot -Directory |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' -and
                [version]$_.Name -ge [version]$Pins['windows-sdk'].version } |
            Sort-Object { [version]$_.Name } -Descending |
            Where-Object {
                $v = $_.Name
                $paths = @("Lib/$v/um/x64/kernel32.lib", "Lib/$v/ucrt/x64/ucrt.lib",
                    "Include/$v/um/Windows.h", "Include/$v/ucrt/stdio.h",
                    "bin/$v/x64/rc.exe", "bin/$v/x64/mt.exe")
                @($paths | Where-Object { -not (Test-Path -LiteralPath (Join-Path $SdkRoot $_) -PathType Leaf) }).Count -eq 0
            } | Select-Object -First 1
    }
    foreach ($instance in @($Instances | Sort-Object { [version]$_.installationVersion } -Descending)) {
        if (-not $instance.isComplete -or [version]$instance.installationVersion -lt [version]'17.0') { continue }
        $vcRoot = Join-Path $instance.installationPath 'VC'
        $versionFile = Join-Path $vcRoot 'Auxiliary/Build/Microsoft.VCToolsVersion.default.txt'
        if (-not (Test-Path -LiteralPath $versionFile -PathType Leaf)) { continue }
        $toolVersion = (Get-Content -Raw -LiteralPath $versionFile).Trim()
        if ($toolVersion -notmatch '^\d+\.\d+\.\d+(\.\d+)?$' -or
            [version]$toolVersion -lt [version]$Pins['msvc-x64-x86'].version) { continue }
        $tools = Join-Path $vcRoot "Tools/MSVC/$toolVersion"
        $paths = @('bin/Hostx64/x64/cl.exe', 'bin/Hostx64/x64/link.exe', 'lib/x64/vcruntime.lib', 'include/vcruntime.h')
        if (@($paths | Where-Object { -not (Test-Path -LiteralPath (Join-Path $tools $_) -PathType Leaf) }).Count -ne 0) { continue }
        if ($sdk) {
            return [pscustomobject]@{ Valid = $true; InstallationPath = $instance.installationPath
                ProductVersion = $instance.installationVersion; MsvcVersion = $toolVersion
                SdkVersion = $sdk.Name; SdkRoot = $SdkRoot }
        }
    }
    return [pscustomobject]@{ Valid = $false; InstallationPath = $null
        ProductVersion = $null; MsvcVersion = $null; SdkVersion = $null; SdkRoot = $SdkRoot }
}

function New-BuildToolsPlan {
    param([object[]]$Instances, [hashtable]$Pins, [string]$InstallerPath)
    # Reuse a Build Tools 2022 instance; never modify the user's full IDE.
    $existing = $Instances | Where-Object { $_.productId -eq $Pins['visual-studio-build-tools'].identifier -and
        $_.isComplete -and [version]$_.installationVersion -ge [version]'17.0' -and
        [version]$_.installationVersion -lt [version]'18.0' } |
        Sort-Object { [version]$_.installationVersion } -Descending | Select-Object -First 1
    $arguments = @('--quiet', '--norestart')
    if ($existing) {
        $channel = if ($existing.PSObject.Properties['channelId'] -and $existing.channelId) {
            $existing.channelId
        } else { $Pins['visual-studio-build-tools'].channel_id }
        $arguments = @('modify', '--installPath', $existing.installationPath,
            '--channelId', $channel) + $arguments
        $action = 'Modify'
    } else { $action = 'Install' }
    foreach ($name in @('msvc-x64-x86', 'windows-sdk')) {
        $arguments += @('--add', $Pins[$name].identifier)
    }
    if (-not $existing) { $arguments += '--wait' }
    return [pscustomobject]@{ Action = $action
        Executable = $(if ($existing) { $InstallerPath } else { 'vs_buildtools.exe' })
        Arguments = $arguments }
}

function Assert-MicrosoftInstaller {
    param($Signature)
    if ($Signature.Status -ne 'Valid' -or -not $Signature.SignerCertificate -or
        $Signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
        throw 'Installer must have a valid Microsoft Corporation Authenticode signature.'
    }
}

function Get-BuildToolsExitStatus {
    param([int]$Code)
    switch ($Code) {
        0 { return 'Completed' }
        3010 { return 'RebootRequired' }
        default { throw "Visual Studio installer failed (exit $Code). Check %TEMP%/dd_* installer logs." }
    }
}

Export-ModuleMember -Function Read-BuildToolsLock, Get-BuildToolsStatus, New-BuildToolsPlan, Assert-MicrosoftInstaller, Get-BuildToolsExitStatus
