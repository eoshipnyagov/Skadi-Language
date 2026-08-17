[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,
    [Parameter(Mandatory = $true)]
    [string]$ChecksumPath,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedVersion,
    [string]$Compiler = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-Checked {
    param(
        [string]$Program,
        [string[]]$Arguments
    )
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "'$Program $($Arguments -join ' ')' failed with exit code $LASTEXITCODE."
    }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) "skadi-dist-smoke-$PID-$([guid]::NewGuid().ToString('N'))"
$installDir = Join-Path $tempRoot "bin"
$manifestPath = Join-Path $tempRoot "state\install-manifest.json"
$projectParent = Join-Path $tempRoot "workspace"
$testHome = Join-Path $tempRoot "home"
$originalHome = $env:HOME
$originalUserProfile = $env:USERPROFILE
$originalPath = $env:PATH
New-Item -ItemType Directory -Path $projectParent -Force | Out-Null
New-Item -ItemType Directory -Path $testHome -Force | Out-Null

try {
    $env:HOME = $testHome
    $env:USERPROFILE = $testHome

    $badChecksums = Join-Path $tempRoot "BAD_SHA256SUMS"
    ("0" * 64) + "  " + (Split-Path -Leaf $ArchivePath) |
        Set-Content -LiteralPath $badChecksums -Encoding ascii
    $checksumRejected = $false
    try {
        & (Join-Path $repoRoot "install\install.ps1") `
            -Version $ExpectedVersion `
            -ArchivePath $ArchivePath `
            -ChecksumPath $badChecksums `
            -InstallDir $installDir `
            -ManifestPath $manifestPath `
            -NoModifyPath
    } catch {
        $checksumRejected = $true
    }
    if (-not $checksumRejected) {
        throw "Installer accepted an invalid checksum."
    }

    & (Join-Path $repoRoot "install\install.ps1") `
        -Version $ExpectedVersion `
        -ArchivePath $ArchivePath `
        -ChecksumPath $ChecksumPath `
        -InstallDir $installDir `
        -ManifestPath $manifestPath `
        -NoModifyPath

    # Reinstalling the same release exercises the atomic upgrade path.
    & (Join-Path $repoRoot "install\install.ps1") `
        -Version $ExpectedVersion `
        -ArchivePath $ArchivePath `
        -ChecksumPath $ChecksumPath `
        -InstallDir $installDir `
        -ManifestPath $manifestPath `
        -NoModifyPath

    $binary = Join-Path $installDir "skadi-cli.exe"
    $env:PATH = "$installDir;$originalPath"
    $actualVersion = (& skadi-cli --version | Out-String).Trim()
    if ($actualVersion -ne "skadi-cli $ExpectedVersion") {
        throw "Unexpected installed version '$actualVersion'."
    }

    Push-Location $projectParent
    try {
        if ($Compiler) {
            $quickSource = Join-Path $projectParent "quick_smoke.skd"
            [System.IO.File]::WriteAllText(
                $quickSource,
                "output(`"installed quick-run ok`")`n",
                [System.Text.UTF8Encoding]::new($false)
            )
            Invoke-Checked "skadi-cli" @("quick-run", $quickSource, "--cc", $Compiler)
        }
        Invoke-Checked "skadi-cli" @("new", "distribution_smoke")
        Set-Location (Join-Path $projectParent "distribution_smoke")
        Invoke-Checked "skadi-cli" @("check")
        Invoke-Checked "skadi-cli" @("format")
        Invoke-Checked "skadi-cli" @("format", "--check")
        Invoke-Checked "skadi-cli" @("doctor")
        Invoke-Checked "skadi-cli" @("target", "list")
        if ($Compiler) {
            Invoke-Checked "skadi-cli" @("build", "--target", "host", "--cc", $Compiler)
            Invoke-Checked "skadi-cli" @("run", "--target", "host", "--cc", $Compiler)
        }
    } finally {
        Pop-Location
    }

    & (Join-Path $repoRoot "install\uninstall.ps1") -ManifestPath $manifestPath
    if (Test-Path -LiteralPath $binary) {
        throw "Uninstaller did not remove $binary."
    }
    if (Test-Path -LiteralPath $manifestPath) {
        throw "Uninstaller did not remove $manifestPath."
    }
} finally {
    $env:PATH = $originalPath
    $env:USERPROFILE = $originalUserProfile
    if ($null -eq $originalHome) {
        Remove-Item Env:HOME -ErrorAction SilentlyContinue
    } else {
        $env:HOME = $originalHome
    }
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
