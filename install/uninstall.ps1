[CmdletBinding()]
param(
    [string]$ManifestPath = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not $ManifestPath) {
    $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
    $ManifestPath = Join-Path $localAppData "Skadi\install-manifest.json"
}
if (-not (Test-Path -LiteralPath $ManifestPath)) {
    Write-Host "Skadi installation manifest not found: $ManifestPath"
    exit 0
}

$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
$binaryPath = [string]$manifest.binary_path
$installDir = [string]$manifest.install_dir

if ($binaryPath -and (Test-Path -LiteralPath $binaryPath)) {
    Remove-Item -LiteralPath $binaryPath -Force
}

if ([bool]$manifest.path_added) {
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($current) {
        $entries = @($current.Split(';') | Where-Object {
            $_.Trim().Length -gt 0 -and
            $_.TrimEnd('\') -ine $installDir.TrimEnd('\')
        })
        [Environment]::SetEnvironmentVariable("Path", ($entries -join ';'), "User")
    }
}

Remove-Item -LiteralPath $ManifestPath -Force

if ($installDir -and (Test-Path -LiteralPath $installDir)) {
    $remaining = @(Get-ChildItem -LiteralPath $installDir -Force)
    if ($remaining.Count -eq 0) {
        Remove-Item -LiteralPath $installDir -Force
    }
}

Write-Host "Skadi CLI uninstalled. User projects and configuration were preserved."
