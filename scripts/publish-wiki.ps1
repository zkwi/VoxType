param(
  # Show how the online wiki differs from docs/wiki without committing or pushing.
  [switch]$DryRun,
  # Commit message; defaults to one derived from the current version.
  [string]$Message = ""
)

$ErrorActionPreference = "Stop"

# Publishes the pages under docs/wiki/ to the GitHub Wiki.
#
# The online wiki is a separate git repository. Merging into the main repository
# never updates it, so it only changes when someone syncs it by hand. The setup
# guide that the app opens on first launch is an online wiki page: once it falls
# behind docs/wiki/, new users are shown outdated setup steps.

if (-not (Test-Path "package.json") -or -not (Test-Path "docs\wiki\Home.md")) {
  Write-Error "Please run this script from the repository root."
}

$wikiUrl = "https://github.com/zkwi/VoxType.wiki.git"
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("voxtype-wiki-" + [System.Guid]::NewGuid().ToString("N"))

$global:LASTEXITCODE = 0
git clone --quiet $wikiUrl $workDir
if ($LASTEXITCODE -ne 0) {
  throw "Failed to clone the wiki repository: $wikiUrl"
}

try {
  # Pages are only added or overwritten; pages that exist only online are kept.
  Copy-Item -Path "docs\wiki\*.md" -Destination $workDir -Force

  $global:LASTEXITCODE = 0
  git -C $workDir -c core.safecrlf=false add --all
  git -C $workDir diff --cached --quiet
  if ($LASTEXITCODE -eq 0) {
    Write-Host "Wiki is already in sync with docs/wiki."
    return
  }

  git -C $workDir diff --cached --stat

  if ($DryRun) {
    Write-Host "Dry run: the wiki differs from docs/wiki; nothing was pushed."
    return
  }

  if (-not $Message) {
    $version = (Get-Content "package.json" -Raw | ConvertFrom-Json).version
    $Message = "Sync wiki with VoxType $version"
  }

  $global:LASTEXITCODE = 0
  git -C $workDir commit --quiet --message $Message
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to commit wiki changes."
  }
  git -C $workDir push --quiet origin HEAD
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to push wiki changes."
  }
  Write-Host "Wiki published."
} finally {
  Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
