$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

Write-Host "Cleaning target..."
cargo clean

Write-Host "Building release..."
cargo build --release

$exe = Join-Path $root "target\\release\\vene_clicker.exe"
if (-not (Test-Path $exe)) {
  throw "Release exe not found at $exe"
}

$tmp = Join-Path $env:TEMP "vene_clicker.exe"
Copy-Item $exe $tmp -Force

Write-Host "Trimming target to exe-only..."
cargo clean
New-Item -ItemType Directory -Force -Path (Join-Path $root "target\\release") | Out-Null
Copy-Item $tmp $exe -Force

Write-Host "Done: $exe"
