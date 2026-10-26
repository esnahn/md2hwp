#requires -Version 7.4
# SPDX-License-Identifier: AGPL-3.0-only
# Isolated helper contracts only: no downloads, installers, registry or machine changes.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Import-Module (Join-Path $repository 'dependencies/windows-build-tools.psm1') -Force
$script:assertions = 0
function Assert-True {
    param([bool]$Condition, [string]$Message)
    $script:assertions++
    if (-not $Condition) { throw "Assertion failed: $Message" }
}
function Assert-Throws {
    param([scriptblock]$Action, [string]$Message)
    $threw = $false
    try { & $Action | Out-Null } catch { $threw = $true }
    Assert-True $threw $Message
}
function Add-FixtureFiles {
    param([string]$Root, [string[]]$Paths)
    foreach ($relative in $Paths) {
        $path = Join-Path $Root $relative
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force
        [IO.File]::WriteAllText($path, '')
    }
}
function New-Instance {
    param([string]$Name, [string]$Version = '17.14.37614.0',
        [string]$ToolVersion = '14.44.35207', [bool]$Complete = $true,
        [string]$Product = 'Microsoft.VisualStudio.Product.BuildTools')
    $path = Join-Path $fixtureRoot $Name
    $versionFile = Join-Path $path 'VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt'
    Add-FixtureFiles $path @('VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt')
    [IO.File]::WriteAllText($versionFile, "$ToolVersion`r`n")
    Add-FixtureFiles (Join-Path $path "VC/Tools/MSVC/$ToolVersion") @(
        'bin/Hostx64/x64/cl.exe', 'bin/Hostx64/x64/link.exe', 'lib/x64/vcruntime.lib', 'include/vcruntime.h')
    return [pscustomobject]@{ installationPath = $path; installationVersion = $Version
        isComplete = $Complete; productId = $Product }
}
function New-Sdk {
    param([string]$Name, [string]$Version = '10.0.26100.0')
    $path = Join-Path $fixtureRoot $Name
    Add-FixtureFiles $path @("Lib/$Version/um/x64/kernel32.lib", "Lib/$Version/ucrt/x64/ucrt.lib",
        "Include/$Version/um/Windows.h", "Include/$Version/ucrt/stdio.h",
        "bin/$Version/x64/rc.exe", "bin/$Version/x64/mt.exe")
    return $path
}
function Write-LockFixture {
    param($Value, [string]$Name)
    $path = Join-Path $fixtureRoot "$Name.json"
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $path -Encoding utf8
    return $path
}
function Read-LockFixture {
    Get-Content -Raw -LiteralPath $lockPath | ConvertFrom-Json
}

$fixtureParent = [IO.Path]::GetFullPath((Join-Path $repository 'target/tests'))
$fixtureRoot = [IO.Path]::GetFullPath((Join-Path $fixtureParent ('windows-build-tools-' + [guid]::NewGuid().ToString('N'))))
$lockPath = Join-Path $repository 'dependencies/lock.json'
$null = New-Item -ItemType Directory -Path $fixtureRoot -Force
try {
    $pins = Read-BuildToolsLock $lockPath
    Assert-True ($pins.Count -eq 3) 'Tracked lock has exactly the three required references'
    $valid = New-Instance 'Build Tools with spaces'
    $sdk = New-Sdk 'Windows Kits'
    $status = Get-BuildToolsStatus @($valid) $pins $sdk
    Assert-True $status.Valid 'Complete compiler and SDK accepted'
    Assert-True ($status.InstallationPath -eq $valid.installationPath) 'Detected path preserved'
    Assert-True ($status.MsvcVersion -eq '14.44.35207' -and $status.SdkVersion -eq '10.0.26100.0') 'Actual tool versions reported'
    Assert-True (-not (Get-BuildToolsStatus @() $pins $sdk).Valid) 'No VS instance rejected'
    Assert-True (-not (Get-BuildToolsStatus @($valid) $pins (Join-Path $fixtureRoot 'missing SDK')).Valid) 'Missing SDK rejected'
    $old = New-Instance 'old compiler' -ToolVersion '14.43.10000'
    Assert-True (-not (Get-BuildToolsStatus @($old) $pins $sdk).Valid) 'Older compiler rejected'
    $oldSdk = New-Sdk 'old SDK' -Version '10.0.22621.0'
    Assert-True (-not (Get-BuildToolsStatus @($valid) $pins $oldSdk).Valid) 'Older SDK rejected'
    $newer = New-Instance 'newer compiler' -Version '18.0.10000.0' -ToolVersion '14.50.10000'
    $newSdk = New-Sdk 'new SDK' -Version '10.0.28000.0'
    Assert-True (Get-BuildToolsStatus @($newer) $pins $newSdk).Valid 'Newer installed toolchain accepted'
    $incomplete = New-Instance 'incomplete' -Complete $false
    Assert-True (-not (Get-BuildToolsStatus @($incomplete) $pins $sdk).Valid) 'Incomplete instance rejected'
    $broken = New-Instance 'missing linker'
    Remove-Item -LiteralPath (Join-Path $broken.installationPath 'VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64/link.exe')
    Assert-True (-not (Get-BuildToolsStatus @($broken) $pins $sdk).Valid) 'Missing linker rejected'
    $brokenSdk = New-Sdk 'missing header SDK'
    Remove-Item -LiteralPath (Join-Path $brokenSdk 'Include/10.0.26100.0/um/Windows.h')
    Assert-True (-not (Get-BuildToolsStatus @($valid) $pins $brokenSdk).Valid) 'Incomplete SDK rejected'
    $status = Get-BuildToolsStatus @($broken, $valid, $newer, $incomplete) $pins $sdk
    Assert-True ($status.Valid -and $status.InstallationPath -eq $newer.installationPath) 'Newest usable instance selected'
    $status = Get-BuildToolsStatus @($broken, $old, $valid) $pins $sdk
    Assert-True ($status.Valid -and $status.InstallationPath -eq $valid.installationPath) 'Bad instances do not hide usable instance'

    $installer = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe'
    $plan = New-BuildToolsPlan @() $pins $installer
    Assert-True ($plan.Action -eq 'Install' -and $plan.Executable -eq 'vs_buildtools.exe') 'New install uses Build Tools bootstrapper'
    Assert-True ($plan.Arguments -contains '--wait') 'Bootstrapper waits for installer'
    Assert-True ($plan.Arguments -contains '--quiet' -and $plan.Arguments -contains '--norestart') 'Silent plan prevents automatic reboot'
    Assert-True (@($plan.Arguments | Where-Object { $_ -eq '--add' }).Count -eq 2) 'Exactly two explicit components requested'
    Assert-True ($plan.Arguments -contains $pins['msvc-x64-x86'].identifier -and
        $plan.Arguments -contains $pins['windows-sdk'].identifier) 'Requested components match lock'
    Assert-True (-not ($plan.Arguments -contains '--all') -and -not ($plan.Arguments -contains '--includeRecommended')) 'No broad workloads added'
    $plan = New-BuildToolsPlan @($valid) $pins $installer
    Assert-True ($plan.Action -eq 'Modify' -and $plan.Executable -eq $installer) 'Existing Build Tools uses installed setup.exe'
    Assert-True ($plan.Arguments[0] -eq 'modify' -and -not ($plan.Arguments -contains '--wait')) 'Installed setup.exe gets modify without bootstrapper-only wait'
    $pathIndex = [array]::IndexOf($plan.Arguments, '--installPath')
    Assert-True ($pathIndex -ge 0 -and $plan.Arguments[$pathIndex + 1] -eq $valid.installationPath) 'Spaced install path remains one argument'
    Assert-True ($plan.Arguments -contains 'VisualStudio.17.Release') 'Modify targets VS 2022 channel'
    $channelInstance = $valid.PSObject.Copy()
    $channelInstance | Add-Member -NotePropertyName channelId -NotePropertyValue 'VisualStudio.17.Release.LTSC.17.12'
    $plan = New-BuildToolsPlan @($channelInstance) $pins $installer
    Assert-True ($plan.Arguments -contains $channelInstance.channelId) 'Modify preserves an existing installation channel'
    $ide = New-Instance 'Full IDE' -Product 'Microsoft.VisualStudio.Product.Community'
    Assert-True (Get-BuildToolsStatus @($ide) $pins $sdk).Valid 'Existing IDE tools may satisfy requirements'
    Assert-True ((New-BuildToolsPlan @($ide) $pins $installer).Action -eq 'Install') 'Incomplete tool requirements never cause full IDE modification'
    $other = New-Instance 'newer Build Tools 2022' -Version '17.15.1.0'
    $plan = New-BuildToolsPlan @($valid, $other, $newer, $ide) $pins $installer
    Assert-True ($plan.Arguments -contains $other.installationPath) 'Modify selects newest Build Tools 2022, ignoring IDE and VS 18'
    Assert-True ((New-BuildToolsPlan @($incomplete) $pins $installer).Action -eq 'Install') 'Incomplete instance is not modified'

    Assert-MicrosoftInstaller ([pscustomobject]@{ Status = 'Valid'; SignerCertificate = [pscustomobject]@{ Subject = 'CN=Microsoft Corporation, O=Microsoft Corporation, C=US' } })
    $script:assertions++
    Assert-Throws { Assert-MicrosoftInstaller ([pscustomobject]@{ Status = 'NotSigned'; SignerCertificate = $null }) } 'Unsigned bootstrapper rejected'
    Assert-Throws { Assert-MicrosoftInstaller ([pscustomobject]@{ Status = 'HashMismatch'; SignerCertificate = [pscustomobject]@{ Subject = 'O=Microsoft Corporation' } }) } 'Invalid Microsoft signature rejected'
    Assert-Throws { Assert-MicrosoftInstaller ([pscustomobject]@{ Status = 'Valid'; SignerCertificate = [pscustomobject]@{ Subject = 'CN=Microsoft Corporation, O=Other Corporation' } }) } 'Microsoft display name alone rejected'
    Assert-Throws { Assert-MicrosoftInstaller ([pscustomobject]@{ Status = 'Valid'; SignerCertificate = [pscustomobject]@{ Subject = 'O=Microsoft Corporation Fake' } }) } 'Microsoft organization prefix spoof rejected'
    Assert-True ((Get-BuildToolsExitStatus 0) -eq 'Completed') 'Exit zero succeeds'
    Assert-True ((Get-BuildToolsExitStatus 3010) -eq 'RebootRequired') 'Exit 3010 requires reboot'
    foreach ($code in @(740, 1001, 1602, 1618, 1641, 5003, 5005, -1)) {
        Assert-Throws { Get-BuildToolsExitStatus $code } "Exit $code does not report ready"
    }

    $bad = Read-LockFixture
    ($bad.dependencies | Where-Object name -eq 'windows-build-tools').pins[0].url = 'https://example.com/vs_buildtools.exe'
    $badPath = Write-LockFixture $bad 'bad-url'
    Assert-Throws { Read-BuildToolsLock $badPath } 'Unofficial bootstrapper URL rejected'
    $bad = Read-LockFixture
    $entry = $bad.dependencies | Where-Object name -eq 'windows-build-tools'
    $entry.pins += $entry.pins[1]
    $badPath = Write-LockFixture $bad 'duplicate-pin'
    Assert-Throws { Read-BuildToolsLock $badPath } 'Duplicate required pin rejected'
    $bad = Read-LockFixture
    $bad.dependencies += ($bad.dependencies | Where-Object name -eq 'windows-build-tools')
    $badPath = Write-LockFixture $bad 'duplicate-dependency'
    Assert-Throws { Read-BuildToolsLock $badPath } 'Duplicate dependency rejected'
    Write-Output "Passed $script:assertions Windows build-tools helper assertions. No installer or download was executed."
} finally {
    # Resolve and check the exact disposable target before any recursive deletion.
    $resolved = [IO.Path]::GetFullPath($fixtureRoot)
    $expectedParent = $fixtureParent.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($expectedParent, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($resolved) -notmatch '^windows-build-tools-[0-9a-f]{32}$') {
        throw "Refusing to clean unexpected fixture directory: $resolved"
    }
    for ($attempt = 0; $attempt -lt 8; $attempt++) {
        try {
            if (Test-Path -LiteralPath $resolved) {
                Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction Stop
            }
            break
        } catch {
            # Windows scanners can briefly hold freshly authored .exe fixtures.
            # Retry sharing/lock violations only; preserve every other cleanup error.
            $failure = $_.Exception
            while ($failure.InnerException) { $failure = $failure.InnerException }
            $nativeCode = $failure.HResult -band 0xffff
            if ($nativeCode -notin @(32, 33) -or $attempt -eq 7) { throw }
            Start-Sleep -Milliseconds (100 * ($attempt + 1))
        }
    }
}
