param(
    [ValidateSet("build", "smoke", "all")]
    [string]$Mode = "all",
    [switch]$StopOnError
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$benches = @(
    @{ name = "bench_01_tree"; file = "benchmarks/bench_01_tree.skd"; args = @("--dirs-only", "--depth-3") },
    @{ name = "bench_02_read_stats"; file = "benchmarks/bench_02_read_stats.skd"; args = @("--input", "examples/example_meteostation.skd") },
    @{ name = "bench_03_find_count"; file = "benchmarks/bench_03_find_count.skd"; args = @("--input", "examples/example_meteostation.skd", "--needle", "temperature") },
    @{ name = "bench_04_sum_ints"; file = "benchmarks/bench_04_sum_ints.skd"; args = @("--medium") },
    @{ name = "bench_05_push_pop"; file = "benchmarks/bench_05_push_pop.skd"; args = @("--medium") },
    @{ name = "bench_06_struct_account"; file = "benchmarks/bench_06_struct_account.skd"; args = @() },
    @{ name = "bench_07_struct_list"; file = "benchmarks/bench_07_struct_list.skd"; args = @() },
    @{ name = "bench_08_path_list_helpers"; file = "benchmarks/bench_08_path_list_helpers.skd"; args = @() }
)

function Invoke-Step {
    param(
        [string]$Label,
        [scriptblock]$Action
    )
    Write-Host "==> $Label"
    & $Action
}

function Build-Bench {
    param($bench)
    $exe = "$($bench.name).exe"
    $exePath = Join-Path $root $exe
    Invoke-Step "Build $($bench.name)" {
        if (Test-Path $exePath) {
            Remove-Item -LiteralPath $exePath -Force
        }
        $cmd = @("--input", $bench.file, "--emit-exe", $exe)
        cargo run -- @cmd
        if (-not (Test-Path $exePath)) {
            throw "Expected executable was not produced: $exePath"
        }
    }
}

function Smoke-Bench {
    param($bench)
    $exePath = Join-Path $root "$($bench.name).exe"
    if (-not (Test-Path $exePath)) {
        throw "Missing executable: $exePath. Run build first."
    }
    Invoke-Step "Run $($bench.name)" {
        & $exePath @($bench.args)
    }
}

$errors = @()
foreach ($bench in $benches) {
    try {
        if ($Mode -in @("build", "all")) {
            Build-Bench $bench
        }
        if ($Mode -in @("smoke", "all")) {
            Smoke-Bench $bench
        }
    } catch {
        $msg = "$($bench.name): $($_.Exception.Message)"
        Write-Host "ERROR: $msg" -ForegroundColor Red
        $errors += $msg
        if ($StopOnError) { break }
    }
}

if ($errors.Count -gt 0) {
    Write-Host ""
    Write-Host "Completed with errors:" -ForegroundColor Yellow
    $errors | ForEach-Object { Write-Host " - $_" -ForegroundColor Yellow }
    exit 1
}

Write-Host ""
Write-Host "Showcase run completed successfully." -ForegroundColor Green
