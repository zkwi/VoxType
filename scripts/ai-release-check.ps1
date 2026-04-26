$ErrorActionPreference = "Stop"

Write-Host "== VoxType AI Release Check =="

function Invoke-CheckedCommand {
  param(
    [string]$Name,
    [scriptblock]$Command
  )

  Write-Host "`n$Name"
  $global:LASTEXITCODE = 0
  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "$Name failed with exit code $LASTEXITCODE"
  }
}

Invoke-CheckedCommand "[1/3] Local checks" { .\scripts\ai-check.ps1 }

Push-Location ".\src-tauri"
try {
  Invoke-CheckedCommand "[2/3] Rust clippy" { cargo clippy --all-targets -- -D warnings }
} finally {
  Pop-Location
}

Invoke-CheckedCommand "[3/3] Tauri debug build" { npx tauri build --debug --no-bundle }

Write-Host "`nRelease checks passed."
