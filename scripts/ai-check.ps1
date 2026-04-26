$ErrorActionPreference = "Stop"

Write-Host "== VoxType AI Local Check =="

if (-not (Test-Path "package.json")) {
  Write-Error "Please run this script from the repository root."
}

Write-Host "`n[1/6] Frontend type check"
npm run check

Write-Host "`n[2/6] Frontend build"
npm run build

Write-Host "`n[3/6] Secret scan"
npm run scan:secrets

Write-Host "`n[4/6] Rust fmt check"
Push-Location ".\src-tauri"
try {
  cargo fmt --check

  Write-Host "`n[5/6] Rust check"
  cargo check

  Write-Host "`n[6/6] Rust tests"
  cargo test
} finally {
  Pop-Location
}

Write-Host "`nAll local checks passed."
