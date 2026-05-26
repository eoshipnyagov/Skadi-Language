param(
    [ValidateSet("game", "embedded", "console", "gui")]
    [string]$Type = "console",
    [switch]$KeepProject,
    [switch]$StopOnError
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$tmpRoot = Join-Path $env:TEMP "skadi_smoke_$stamp"
$projName = "demo_$Type"
$projPath = Join-Path $tmpRoot $projName
$errors = New-Object System.Collections.Generic.List[string]

function Invoke-Step {
    param(
        [string]$Label,
        [scriptblock]$Action
    )
    Write-Host "==> $Label"
    & $Action
}

function Run-Cmd {
    param(
        [string]$Label,
        [string[]]$Command
    )
    try {
        Invoke-Step $Label {
            & $Command[0] $Command[1..($Command.Length - 1)]
        }
    } catch {
        $msg = "$Label failed: $($_.Exception.Message)"
        Write-Host "ERROR: $msg" -ForegroundColor Red
        $errors.Add($msg) | Out-Null
        if ($StopOnError) { throw }
    }
}

New-Item -ItemType Directory -Path $tmpRoot -Force | Out-Null

Run-Cmd "CLI help" @("cargo", "run", "--manifest-path", "tools/skadi-cli/Cargo.toml", "--", "help")
Run-Cmd "Create project" @("cargo", "run", "--manifest-path", "tools/skadi-cli/Cargo.toml", "--", "new", $Type, $projPath)

if (-not (Test-Path $projPath)) {
    throw "Project folder was not created: $projPath"
}

Push-Location $projPath
try {
    Run-Cmd "Check project" @("cargo", "run", "--manifest-path", "$root/tools/skadi-cli/Cargo.toml", "--", "check")
    Run-Cmd "Build project" @("cargo", "run", "--manifest-path", "$root/tools/skadi-cli/Cargo.toml", "--", "build")
    Run-Cmd "Run project" @("cargo", "run", "--manifest-path", "$root/tools/skadi-cli/Cargo.toml", "--", "run")
    Run-Cmd "Generate examples" @("cargo", "run", "--manifest-path", "$root/tools/skadi-cli/Cargo.toml", "--", "examples")
    Run-Cmd "Clean project artifacts" @("cargo", "run", "--manifest-path", "$root/tools/skadi-cli/Cargo.toml", "--", "clean")
} finally {
    Pop-Location
}

if ($errors.Count -gt 0) {
    Write-Host ""
    Write-Host "Smoke from-zero completed with errors:" -ForegroundColor Yellow
    $errors | ForEach-Object { Write-Host " - $_" -ForegroundColor Yellow }
    if (-not $KeepProject -and (Test-Path $tmpRoot)) {
        Remove-Item -LiteralPath $tmpRoot -Recurse -Force
    }
    exit 1
}

Write-Host ""
Write-Host "Smoke from-zero completed successfully for type '$Type'." -ForegroundColor Green
Write-Host "Project path: $projPath"

if (-not $KeepProject -and (Test-Path $tmpRoot)) {
    Remove-Item -LiteralPath $tmpRoot -Recurse -Force
}
