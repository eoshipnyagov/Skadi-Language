param(
    [string]$InstallRoot = "$env:LOCALAPPDATA\\Skadi",
    [switch]$NoPathUpdate
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$binDir = Join-Path $InstallRoot "bin"

Write-Host "==> Installing skadi-cli to: $InstallRoot"
cargo install --path tools/skadi-cli --root $InstallRoot --force

$skadiCmd = Join-Path $binDir "skadi.cmd"
@"
@echo off
"$binDir\skadi-cli.exe" %*
"@ | Set-Content -Path $skadiCmd -Encoding ASCII

$env:Path = "$binDir;$env:Path"

if (-not $NoPathUpdate) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) {
        $userPath = ""
    }
    $parts = $userPath -split ";" | Where-Object { $_ -ne "" }
    if (-not ($parts -contains $binDir)) {
        $newPath = if ($userPath.Trim().Length -eq 0) { $binDir } else { "$userPath;$binDir" }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "==> Added to user PATH: $binDir"
    } else {
        Write-Host "==> PATH already contains: $binDir"
    }
} else {
    Write-Host "==> Skipping PATH update (--NoPathUpdate)."
}

Write-Host "==> Verifying skadi command..."
& "$binDir\skadi.cmd" help

Write-Host ""
Write-Host "Install complete."
Write-Host "If your current terminal does not see 'skadi', open a new terminal session."
