#requires -Version 7.0

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Find-CommandPath([string]$Name) {
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($null -eq $command) {
        return $null
    }
    return $command.Source
}

$hancomComRegistered = $false
$securityRegistryPath = $null
$securityRegistryTarget = $null
$securityDllExists = $false
$dotnetPath = Find-CommandPath "dotnet"
$dotnetSdks = @()

if ($null -ne $dotnetPath) {
    $dotnetSdkRoot = Join-Path (Split-Path -Parent $dotnetPath) "sdk"
    if (Test-Path -LiteralPath $dotnetSdkRoot -PathType Container) {
        $dotnetSdks = @(
            Get-ChildItem -LiteralPath $dotnetSdkRoot -Directory |
                Select-Object -ExpandProperty Name
        )
    }
}

if ($IsWindows) {
    $hancomComRegistered = $null -ne [Type]::GetTypeFromProgID("HWPFrame.HwpObject")
    $securityRegistryPath = "HKCU:\Software\HNC\HwpAutomation\Modules"
    if (Test-Path -LiteralPath $securityRegistryPath) {
        $entry = Get-ItemProperty -LiteralPath $securityRegistryPath `
            -Name "FilePathCheckerModuleExample" -ErrorAction SilentlyContinue
        if ($null -ne $entry) {
            $securityRegistryTarget = $entry.FilePathCheckerModuleExample
            $securityDllExists = Test-Path -LiteralPath $securityRegistryTarget -PathType Leaf
        }
    }
}

[pscustomobject]@{
    User = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    Sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    SessionId = [Diagnostics.Process]::GetCurrentProcess().SessionId
    PSEdition = $PSVersionTable.PSEdition
    Is64BitProcess = [Environment]::Is64BitProcess
    ApartmentState = [Threading.Thread]::CurrentThread.ApartmentState
    UserInteractive = [Environment]::UserInteractive
    LocalApplicationData = [Environment]::GetFolderPath("LocalApplicationData")
    OperatingSystem = [Environment]::OSVersion.VersionString
    PowerShell = $PSVersionTable.PSVersion.ToString()
    Pwsh = Find-CommandPath "pwsh"
    WindowsPowerShell = Find-CommandPath "powershell.exe"
    Rustc = Find-CommandPath "rustc"
    Cargo = Find-CommandPath "cargo"
    DotNet = $dotnetPath
    DotNetSdks = $dotnetSdks
    Pandoc = Find-CommandPath "pandoc"
    HancomComRegistered = $hancomComRegistered
    SecurityRegistryPath = $securityRegistryPath
    SecurityRegistryTarget = $securityRegistryTarget
    SecurityDllExists = $securityDllExists
}
