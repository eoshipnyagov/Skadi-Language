param(
    [switch]$All
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$removed = New-Object System.Collections.Generic.List[string]

function Remove-DirIfExists {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
        $removed.Add($Path) | Out-Null
    }
}

function Remove-FileMatches {
    param([string]$Pattern)
    Get-ChildItem -LiteralPath $root -File -Filter $Pattern | ForEach-Object {
        Remove-Item -LiteralPath $_.FullName -Force
        $removed.Add($_.FullName) | Out-Null
    }
}

Remove-DirIfExists (Join-Path $root "build")
Remove-FileMatches "bench_*.exe"
Remove-FileMatches "*.scadi.c"

if ($All) {
    Remove-DirIfExists (Join-Path $root "target")
    Remove-DirIfExists (Join-Path $root "tools/skadi-cli/target")
}

if ($removed.Count -eq 0) {
    Write-Host "clean: nothing to remove"
} else {
    $removed | ForEach-Object { Write-Host "clean: removed $_" }
}
