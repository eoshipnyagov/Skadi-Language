[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$InstallDir = "",
    [string]$ManifestPath = "",
    [string]$Repository = "eoshipnyagov/Skadi-Language",
    [string]$ArchivePath = "",
    [string]$ChecksumPath = "",
    [switch]$NoModifyPath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-LatestReleaseTag {
    param([string]$Repo)

    $headers = @{
        "Accept" = "application/vnd.github+json"
        "User-Agent" = "skadi-cli-installer"
    }
    $release = Invoke-RestMethod `
        -Uri "https://api.github.com/repos/$Repo/releases/latest" `
        -Headers $headers
    if (-not $release.tag_name) {
        throw "GitHub latest release response does not contain tag_name."
    }
    return [string]$release.tag_name
}

function Normalize-ReleaseVersion {
    param([string]$RawVersion)

    $tag = if ($RawVersion.StartsWith("v")) { $RawVersion } else { "v$RawVersion" }
    $clean = $tag.Substring(1)
    if ($clean -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
        throw "Invalid Skadi release version '$RawVersion'."
    }
    return @{
        Tag = $tag
        Version = $clean
    }
}

function Save-RemoteFile {
    param(
        [string]$Uri,
        [string]$Destination
    )

    $headers = @{ "User-Agent" = "skadi-cli-installer" }
    Invoke-WebRequest -Uri $Uri -Headers $headers -OutFile $Destination -UseBasicParsing
}

function Assert-Checksum {
    param(
        [string]$File,
        [string]$Manifest
    )

    $name = Split-Path -Leaf $File
    $escapedName = [regex]::Escape($name)
    $line = Get-Content -LiteralPath $Manifest |
        Where-Object { $_.Trim() -match "^([0-9A-Fa-f]{64})\s+\*?$escapedName$" } |
        Select-Object -First 1
    if (-not $line) {
        throw "Checksum manifest does not contain '$name'."
    }
    $expected = ($line.Trim() -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "Checksum mismatch for '$name': expected $expected, got $actual."
    }
}

function Add-UserPathEntry {
    param([string]$Directory)

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = @()
    if ($current) {
        $entries = @($current.Split(';') | Where-Object { $_.Trim().Length -gt 0 })
    }
    $alreadyPresent = $entries | Where-Object {
        $_.TrimEnd('\') -ieq $Directory.TrimEnd('\')
    }
    if ($alreadyPresent) {
        return $false
    }
    $updated = (@($entries) + $Directory) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $updated, "User")
    if (-not (($env:Path -split ';') | Where-Object {
        $_.TrimEnd('\') -ieq $Directory.TrimEnd('\')
    })) {
        $env:Path = "$Directory;$env:Path"
    }
    return $true
}

function Install-BinaryAtomically {
    param(
        [string]$StagedPath,
        [string]$BinaryPath,
        [string]$InstallDir
    )

    if (-not (Test-Path -LiteralPath $BinaryPath)) {
        Move-Item -LiteralPath $StagedPath -Destination $BinaryPath
        return
    }

    $backupPath = Join-Path $InstallDir ".skadi-cli.backup.$PID.exe"
    for ($attempt = 1; $attempt -le 10; $attempt++) {
        try {
            [IO.File]::Replace($StagedPath, $BinaryPath, $backupPath, $true)
            Remove-Item -LiteralPath $backupPath -Force -ErrorAction SilentlyContinue
            return
        } catch [IO.IOException] {
            if ($attempt -eq 10) {
                throw
            }
            Start-Sleep -Milliseconds 200
        }
    }
}

if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)) {
    throw "install.ps1 supports Windows only. Use install/install.sh on Unix."
}

if (-not $InstallDir) {
    $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
    if (-not $localAppData) {
        throw "Unable to determine LocalApplicationData."
    }
    $InstallDir = Join-Path $localAppData "Programs\Skadi\bin"
}
if (-not $ManifestPath) {
    $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
    $ManifestPath = Join-Path $localAppData "Skadi\install-manifest.json"
}

$previousManifest = $null
if (Test-Path -LiteralPath $ManifestPath) {
    $previousManifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    $previousInstallDir = [string]$previousManifest.install_dir
    if ($previousInstallDir -and $previousInstallDir.TrimEnd('\') -ine $InstallDir.TrimEnd('\')) {
        throw "Skadi is already managed at '$previousInstallDir'. Uninstall it before changing -InstallDir."
    }
}

$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
if ($architecture -ne "X64") {
    throw "Unsupported Windows architecture '$architecture'. v1.2 RC provides x86-64 binaries."
}

$resolvedVersion = if ($Version -eq "latest") {
    Normalize-ReleaseVersion (Get-LatestReleaseTag $Repository)
} else {
    Normalize-ReleaseVersion $Version
}
$tag = $resolvedVersion.Tag
$cleanVersion = $resolvedVersion.Version
$target = "x86_64-pc-windows-msvc"
$archiveName = "skadi-cli-v$cleanVersion-$target.zip"

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "skadi-install-$PID-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null

try {
    if ($ArchivePath) {
        if (-not $ChecksumPath) {
            throw "-ChecksumPath is required with -ArchivePath."
        }
        $archive = (Resolve-Path -LiteralPath $ArchivePath).Path
        $checksums = (Resolve-Path -LiteralPath $ChecksumPath).Path
        if ((Split-Path -Leaf $archive) -ne $archiveName) {
            throw "Local archive must be named '$archiveName'."
        }
    } else {
        $archive = Join-Path $tempRoot $archiveName
        $checksums = Join-Path $tempRoot "SHA256SUMS"
        $baseUrl = "https://github.com/$Repository/releases/download/$tag"
        Write-Host "Downloading $archiveName"
        Save-RemoteFile "$baseUrl/$archiveName" $archive
        Save-RemoteFile "$baseUrl/SHA256SUMS" $checksums
    }

    Assert-Checksum $archive $checksums

    $extractDir = Join-Path $tempRoot "extract"
    Expand-Archive -LiteralPath $archive -DestinationPath $extractDir -Force
    $sourceBinary = Get-ChildItem -LiteralPath $extractDir -Recurse -File -Filter "skadi-cli.exe" |
        Select-Object -First 1
    if (-not $sourceBinary) {
        throw "Archive '$archiveName' does not contain skadi-cli.exe."
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $binaryPath = Join-Path $InstallDir "skadi-cli.exe"
    $stagedPath = Join-Path $InstallDir ".skadi-cli.installing.$PID.exe"
    Copy-Item -LiteralPath $sourceBinary.FullName -Destination $stagedPath -Force

    $stagedVersion = (& $stagedPath --version 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $stagedVersion -ne "skadi-cli $cleanVersion") {
        Remove-Item -LiteralPath $stagedPath -Force -ErrorAction SilentlyContinue
        throw "Downloaded binary failed version verification: '$stagedVersion'."
    }

    Install-BinaryAtomically $stagedPath $binaryPath $InstallDir

    $pathAdded = $previousManifest -and [bool]$previousManifest.path_added
    if (-not $NoModifyPath -and -not $pathAdded) {
        $pathAdded = Add-UserPathEntry $InstallDir
    }

    $manifestDir = Split-Path -Parent $ManifestPath
    New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
    $manifestTemp = "$ManifestPath.tmp.$PID"
    @{
        schema_version = 1
        version = $cleanVersion
        install_dir = $InstallDir
        binary_path = $binaryPath
        path_added = [bool]$pathAdded
    } | ConvertTo-Json | Set-Content -LiteralPath $manifestTemp -Encoding utf8
    Move-Item -LiteralPath $manifestTemp -Destination $ManifestPath -Force

    & $binaryPath --version
    & $binaryPath doctor
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "skadi-cli doctor reported an incomplete host toolchain."
    }

    Write-Host ""
    Write-Host "Skadi CLI $cleanVersion installed to $binaryPath"
    if ($NoModifyPath) {
        Write-Host "PATH was not modified. Add '$InstallDir' to your user PATH."
    } elseif ($pathAdded) {
        Write-Host "User PATH updated. Open a new terminal before running skadi-cli."
    }
} finally {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
