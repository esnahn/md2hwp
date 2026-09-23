# Embedded by the Rust application. No runtime lock.json or administrator access.
param([string]$InstallRoot)
& {
    $ErrorActionPreference = 'Stop'
    Set-StrictMode -Version Latest
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $stage = $null
    try {
        $root = if ($InstallRoot) { $InstallRoot } else { Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'md2hwp\pandoc' }
        $root = [IO.Path]::GetFullPath($root)
        $null = [IO.Directory]::CreateDirectory($root)
        $stage = Join-Path $root ('.download-' + [guid]::NewGuid().ToString('N'))
        $null = [IO.Directory]::CreateDirectory($stage)
        $archive = Join-Path $stage 'upstream.zip'
        $version = $env:MD2HWP_PANDOC_VERSION
        $url = $env:MD2HWP_PANDOC_URL
        $digest = $env:MD2HWP_PANDOC_SHA256
        if ($version -notmatch '^\d+(\.\d+){1,3}$' -or $digest -notmatch '^[A-Fa-f0-9]{64}$' -or
            $url -cne "https://github.com/jgm/pandoc/releases/download/$version/pandoc-$version-windows-x86_64.zip") {
            throw 'Invalid embedded Pandoc download defaults.'
        }
        Write-Output "Downloading preferred Pandoc $version from upstream..."
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive -TimeoutSec 120
        } catch {
            Write-Output 'Preferred release unavailable; looking up the latest official stable release.'
            $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/jgm/pandoc/releases/latest' -Headers @{ 'User-Agent'='md2hwp-setup' } -TimeoutSec 30
            $version = [string]$release.tag_name
            if ($release.draft -or $release.prerelease -or $version -notmatch '^\d+(\.\d+){1,3}$') { throw 'Invalid stable release metadata.' }
            $assetName = "pandoc-$version-windows-x86_64.zip"
            $assets = @($release.assets | Where-Object { $_.name -ceq $assetName })
            if ($assets.Count -ne 1) { throw 'No unique official Windows x64 Pandoc archive.' }
            $url = [string]$assets[0].browser_download_url
            if ($url -cne "https://github.com/jgm/pandoc/releases/download/$version/$assetName") { throw 'Unexpected upstream download URL.' }
            if (-not $assets[0].PSObject.Properties['digest'] -or $assets[0].digest -notmatch '^sha256:([a-fA-F0-9]{64})$') {
                throw 'Official release did not provide a SHA-256 digest; use https://pandoc.org/installing.html instead.'
            }
            $digest = $Matches[1]
            Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive -TimeoutSec 120
        }
        if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ine $digest) {
            throw 'Downloaded archive integrity check failed. No fallback or installation performed.'
        }
        $expanded = Join-Path $stage 'upstream'
        Expand-Archive -LiteralPath $archive -DestinationPath $expanded
        $executables = @(Get-ChildItem -LiteralPath $expanded -Recurse -File -Filter pandoc.exe)
        if ($executables.Count -ne 1) { throw 'Expected one pandoc.exe in the official archive.' }
        # Probe compatibility, not an exact executable version. Never run a downloaded installer.
        $json = '# md2hwp compatibility check' | & $executables[0].FullName '--from=commonmark' '--to=json'
        if ($LASTEXITCODE -ne 0) { throw 'Downloaded Pandoc could not produce CommonMark JSON.' }
        $probe = ($json -join "`n") | ConvertFrom-Json
        if (($probe.'pandoc-api-version' -join '.') -ne '1.23.1.2' -or $probe.blocks[0].t -ne 'Header') {
            throw 'Downloaded Pandoc JSON format is not supported by this md2hwp build. Existing installation retained.'
        }
        # Preserve the entire upstream archive/documentation and its release-specific notice.
        $copyrightUrl = "https://raw.githubusercontent.com/jgm/pandoc/$version/COPYRIGHT"
        Invoke-WebRequest -UseBasicParsing -Uri $copyrightUrl -OutFile (Join-Path $stage 'COPYRIGHT') -TimeoutSec 30
        $notice = "Pandoc - John MacFarlane and contributors`r`nGPL-2.0-or-later; see COPYRIGHT and upstream notices.`r`nBinary: $url`r`nSource: https://github.com/jgm/pandoc/tree/$version`r`nInstallation: https://pandoc.org/installing.html`r`n"
        [IO.File]::WriteAllText((Join-Path $stage 'UPSTREAM.txt'), $notice)
        $relativeExe = $executables[0].FullName.Substring($stage.Length + 1)
        $destinationName = $version + '-' + [guid]::NewGuid().ToString('N')
        $destination = Join-Path $root $destinationName
        # Both paths are immediate children of the documented application cache.
        if ([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($stage)) -ine $root -or
            [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($destination)) -ine $root) { throw 'Install path escaped cache.' }
        [IO.Directory]::Move($stage, $destination)
        $stage = $null
        $pointer = Join-Path $root 'current.txt'
        $temporaryPointer = Join-Path $root ('current-' + [guid]::NewGuid().ToString('N') + '.tmp')
        try {
            [IO.File]::WriteAllText($temporaryPointer, (Join-Path $destinationName $relativeExe))
            if ([IO.File]::Exists($pointer)) { [IO.File]::Replace($temporaryPointer, $pointer, $null) }
            else { [IO.File]::Move($temporaryPointer, $pointer) }
        } finally {
            if ([IO.File]::Exists($temporaryPointer)) { [IO.File]::Delete($temporaryPointer) }
        }
        Write-Output "Installed Pandoc ${version}: $(Join-Path $destination $relativeExe)"
        Write-Output 'Original archive, documentation, COPYRIGHT and source link retained. System PATH unchanged.'
    } catch {
        [Console]::Error.WriteLine($_.Exception.Message)
        exit 1
    } finally {
        if ($stage -and [IO.Directory]::Exists($stage)) {
            $absoluteStage = [IO.Path]::GetFullPath($stage)
            if ([IO.Path]::GetDirectoryName($absoluteStage) -ine $root -or
                [IO.Path]::GetFileName($absoluteStage) -notmatch '^\.download-[0-9a-f]{32}$') { throw 'Unsafe staging cleanup rejected.' }
            Remove-Item -LiteralPath $absoluteStage -Recurse -Force
        }
    }
}
