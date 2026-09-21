<#
.SYNOPSIS
    Installs Quiver Desktop on Windows.

.DESCRIPTION
    Run it with:

        irm https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/develop/install.ps1 | iex

    This downloads the NSIS installer from the latest GitHub release and runs
    it silently. The NSIS bundle is the one it picks, never the .msi published
    alongside it: NSIS defaults to installMode "currentUser"
    (RequestExecutionLevel user, $INSTDIR = $LOCALAPPDATA\Quiver), so nothing
    here needs administrator rights, while the WiX .msi installs per-machine
    and would. ARROW.md's own install lifecycle makes the same choice, so a
    script install and an arrow install put the same thing in the same place,
    and quiver.core's preinstalled probe recognises either.

    Targets Windows PowerShell 5.1 as well as PowerShell 7: no ternaries, no
    null-coalescing, and no $IsWindows, none of which exist in 5.1.

.NOTES
    Piped into iex there is nowhere for arguments to bind, so the knobs are
    environment variables, read by the call at the bottom of this file:

        QUIVER_TAG                  install this release tag, not the latest
        QUIVER_INSTALL_SOURCE_ONLY  set to 1 to define the functions and run
                                    nothing, for dot-sourcing in tests

    Dot-source the file instead and Install-QuiverDesktop takes -Tag,
    -Repository and -ApiBaseUrl as ordinary parameters.
#>

function Get-QuiverReleaseUrl {
    [CmdletBinding()]
    param(
        [string] $Repository = 'rabbytesoftware/quiver.desktop',
        [string] $ApiBaseUrl = 'https://api.github.com',
        [string] $Tag = ''
    )

    if ([string]::IsNullOrWhiteSpace($Tag)) {
        return "$ApiBaseUrl/repos/$Repository/releases/latest"
    }
    return "$ApiBaseUrl/repos/$Repository/releases/tags/$Tag"
}

function Get-QuiverArchPattern {
    [CmdletBinding()]
    param([string] $Architecture)

    switch -Regex ($Architecture) {
        '^(AMD64|x64|x86_64)$' { return '(x64|x86_64|amd64)' }
        '^(ARM64|aarch64)$' { return '(arm64|aarch64)' }
        default { return '.' }
    }
}

<#
.SYNOPSIS
    Picks the NSIS setup executable to download out of a release object.
.DESCRIPTION
    Selection is by suffix, never by full filename: quiver.desktop's release
    filenames carry tauri.conf.json's static "0.1.0", which does not track the
    git tag a release is cut from, so no name can be predicted from the tag
    being installed.

    When several setup executables are published, the one naming this machine's
    architecture wins; when none names it, the first is used, which is correct
    for a release that ships a single installer.
#>
function Select-QuiverAssetUrl {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)] $Release,
        [string] $Architecture = 'AMD64'
    )

    $candidates = @($Release.assets | Where-Object { $_.name -match '-setup\.exe$' })
    if ($candidates.Count -eq 0) {
        throw "This release publishes no *-setup.exe asset. Nothing to install."
    }

    $pattern = Get-QuiverArchPattern -Architecture $Architecture
    $narrowed = @($candidates | Where-Object { $_.name -match $pattern })
    if ($narrowed.Count -gt 0) {
        $candidates = $narrowed
    }

    return $candidates[0].browser_download_url
}

<#
.SYNOPSIS
    Reads the sha256 GitHub recorded for an asset, or $null when it has none.
.DESCRIPTION
    The releases API reports a "digest" per asset -- GitHub's own record of
    what it stored, written as "sha256:<hex>". It needs no extra request and
    needs nobody to have published a checksum manifest, which is why the
    caller tries it first: it is what lets a quiver.desktop release verify at
    all, since this repository's release workflow publishes no manifest.

    An asset from before GitHub recorded digests carries $null here, which is
    what the manifest lookup below is still for. Anything that is not a
    64-character sha256 hex is discarded rather than passed on as something
    that would then fail to match for the wrong reason.
#>
function Get-QuiverAssetDigest {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)] $Release,
        [Parameter(Mandatory = $true)] [string] $AssetUrl
    )

    $asset = @($Release.assets | Where-Object { $_.browser_download_url -eq $AssetUrl })
    if ($asset.Count -eq 0) { return $null }

    $digest = $asset[0].digest
    if ([string]::IsNullOrWhiteSpace($digest)) { return $null }
    if ($digest -notmatch '^sha256:([0-9a-fA-F]{64})$') { return $null }
    return $Matches[1].ToLowerInvariant()
}

<#
.SYNOPSIS
    Finds the release's checksum manifest, or returns $null when there is none.
.DESCRIPTION
    .github/workflows/stable-release.yml uploads the bundle files and nothing
    else, so no release publishes a checksum manifest today and this returns
    $null. It stays because publishing one later turns this route on with no
    change here, and because it is the only route for an asset that predates
    GitHub recording digests of its own.
#>
function Select-QuiverChecksumsUrl {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)] $Release)

    $found = @($Release.assets | Where-Object {
            $_.name -match '^(checksums(\.txt)?|SHA256SUMS(\.txt)?)$'
        })
    if ($found.Count -eq 0) {
        return $null
    }
    return $found[0].browser_download_url
}

<#
.SYNOPSIS
    Reads the digest recorded for $FileName out of a sha256sum-format manifest.
.DESCRIPTION
    Handles both forms sha256sum writes: "<hash>  name" and "<hash> *name".
    Returns $null when the manifest carries no line for that name.
#>
function Get-QuiverExpectedSum {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)] [string] $Manifest,
        [Parameter(Mandatory = $true)] [string] $FileName
    )

    foreach ($line in $Manifest -split "\r?\n") {
        $fields = $line.Trim() -split '\s+', 2
        if ($fields.Count -lt 2) { continue }
        $name = $fields[1].Trim().TrimStart('*')
        if ($name -eq $FileName) {
            return $fields[0].Trim()
        }
    }
    return $null
}

function Install-QuiverDesktop {
    [CmdletBinding()]
    param(
        [string] $Tag = '',
        [string] $Repository = 'rabbytesoftware/quiver.desktop',
        [string] $ApiBaseUrl = 'https://api.github.com'
    )

    # Scoped to this function, so an `irm | iex` does not leave the caller's
    # session running under settings it never asked for.
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'

    # $IsWindows does not exist in Windows PowerShell 5.1, where this one-liner
    # most often runs. $env:OS is the check that works in both.
    if ($env:OS -ne 'Windows_NT') {
        throw "install.ps1 installs on Windows only. On macOS or Linux use:`n    curl -fsSL https://raw.githubusercontent.com/$Repository/develop/install.sh | bash"
    }

    # Windows PowerShell 5.1 still negotiates TLS 1.0 by default on older
    # machines, which api.github.com refuses outright.
    try {
        $tls12 = [Net.SecurityProtocolType]::Tls12
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor $tls12
    }
    catch {
        Write-Verbose "Could not raise the TLS version: $_"
    }

    $architecture = $env:PROCESSOR_ARCHITEW6432
    if ([string]::IsNullOrWhiteSpace($architecture)) {
        $architecture = $env:PROCESSOR_ARCHITECTURE
    }
    Write-Host "Installing Quiver Desktop for windows/$architecture..."

    $releaseUrl = Get-QuiverReleaseUrl -Repository $Repository -ApiBaseUrl $ApiBaseUrl -Tag $Tag
    try {
        $release = Invoke-RestMethod -Uri $releaseUrl -UseBasicParsing -Headers @{
            'Accept'     = 'application/vnd.github+json'
            'User-Agent' = 'quiver-desktop-installer'
        }
    }
    catch {
        throw "Could not read the release list from GitHub ($releaseUrl): $($_.Exception.Message)"
    }

    $assetUrl = Select-QuiverAssetUrl -Release $release -Architecture $architecture
    $fileName = [System.IO.Path]::GetFileName(([System.Uri] $assetUrl).AbsolutePath)

    $workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("quiver-install-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $workDir -Force | Out-Null
    $installer = Join-Path $workDir $fileName

    try {
        Write-Host "Downloading $fileName..."
        try {
            Invoke-WebRequest -Uri $assetUrl -OutFile $installer -UseBasicParsing
        }
        catch {
            throw "Download failed ($assetUrl): $($_.Exception.Message)"
        }

        # GitHub's own digest first, a published manifest second. Nothing
        # published anywhere is reported and accepted: the download still came
        # from GitHub over TLS, and refusing here would leave the user with no
        # app at all. Quiver's in-app Update button reads the same two sources
        # in the same order and then REFUSES what it cannot verify, because a
        # fetch step cannot be told to skip verification and because refusing
        # an update leaves a working app on screen. Same inputs, different
        # stakes.
        $expected = Get-QuiverAssetDigest -Release $release -AssetUrl $assetUrl
        if ($null -eq $expected) {
            $checksumsUrl = Select-QuiverChecksumsUrl -Release $release
            if ($null -eq $checksumsUrl) {
                Write-Warning "This release publishes no checksum for $fileName, so the download could not be verified beyond TLS."
            }
            else {
                $manifest = (Invoke-WebRequest -Uri $checksumsUrl -UseBasicParsing).Content
                $expected = Get-QuiverExpectedSum -Manifest $manifest -FileName $fileName
                if ($null -eq $expected) {
                    Write-Warning "The checksum manifest has no entry for $fileName; skipping verification."
                }
            }
        }

        if ($null -ne $expected) {
            $actual = (Get-FileHash -Path $installer -Algorithm SHA256).Hash
            if ($actual -ne $expected.ToUpperInvariant()) {
                throw "Checksum mismatch for ${fileName}: expected $expected, got $actual"
            }
            Write-Host "Checksum verified."
        }

        Write-Host "Running the installer..."
        $process = Start-Process -FilePath $installer -ArgumentList '/S' -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            throw "The installer exited with code $($process.ExitCode)."
        }
    }
    finally {
        Remove-Item -Path $workDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    $installDir = Join-Path $env:LOCALAPPDATA 'Quiver'
    Write-Host "Installed to $installDir"
    Write-Host "Open it with: Start-Process `"$installDir\quiverdesktop.exe`""
    Write-Host "Done."
}

# Dot-sourcing this file with QUIVER_INSTALL_SOURCE_ONLY=1 defines every
# function and runs nothing, so asset selection and checksum matching can be
# exercised without touching the network.
if ($env:QUIVER_INSTALL_SOURCE_ONLY -ne '1') {
    Install-QuiverDesktop -Tag $env:QUIVER_TAG
}
