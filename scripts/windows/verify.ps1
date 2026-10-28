# Runs the pre-PR checks of FormaCAD on Windows and, on request, builds the installer.
# Usage (from any directory):
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/verify.ps1 [-Install] [-Installer -OcctRoot <dir>]
param(
    # Run `npm ci` first (clean dependency install).
    [switch]$Install,
    # Build the Windows installer after all checks pass. Requires -OcctRoot.
    [switch]$Installer,
    [string]$OcctRoot = '',
    # Shared Cargo target directory, for example one reused by several worktrees.
    [string]$CargoTargetDir = '',
    # File with the updater signing key. Must live outside Git (for example `.secrets/updater.key`).
    # Without it the existing TAURI_SIGNING_PRIVATE_KEY environment variable is used.
    [string]$UpdaterKeyFile = '',
    # Directory for temporary files (TMP/TEMP) of this run, for example on a larger disk.
    [string]$TempDir = ''
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$tauri = Join-Path $root 'apps\desktop\src-tauri'
$desktop = Join-Path $root 'apps\desktop'

if ($TempDir) {
    New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
    $env:TMP = $TempDir; $env:TEMP = $TempDir
}
if ($CargoTargetDir) { $env:CARGO_TARGET_DIR = $CargoTargetDir }
. (Join-Path $root 'scripts\windows-env.ps1') | Out-Null

function Step($name, [scriptblock]$block) {
    Write-Host "=== $name"
    & $block
    if ($LASTEXITCODE -ne 0) { Write-Host "FAILED: $name (exit $LASTEXITCODE)"; exit 1 }
}

Set-Location -LiteralPath $root
if ($Install) { Step 'npm ci' { npm ci --no-audit --no-fund } }
Step 'version' { node scripts/version.mjs }
Step 'i18n' { node scripts/check-i18n.mjs }
Step 'docs' { node scripts/check-docs.mjs }
Step 'migrations' { node scripts/check-migrations.mjs }

Set-Location -LiteralPath $desktop
Step 'eslint' { npx eslint src }
Step 'typecheck + build' { npm run build }
Step 'vitest' { npx vitest run }

Set-Location -LiteralPath $tauri
Step 'cargo fmt' { cargo fmt --check }
Step 'cargo clippy' { cargo clippy --all-targets -- -D warnings }
Step 'cargo test' { cargo test --lib }

if ($Installer) {
    if (-not $OcctRoot) { throw '-Installer requires -OcctRoot (for example .local\occt-build).' }
    if ($UpdaterKeyFile) {
        # The key stays in this process only; it is never written or printed.
        $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw -LiteralPath $UpdaterKeyFile
        if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '' }
    }
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY) { Write-Host 'WARNING: no updater signing key; update signatures will not be produced.' }
    Set-Location -LiteralPath $root
    Step 'installer' { powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-native-windows.ps1 -OcctRoot $OcctRoot }
}

Write-Host '=== ALL OK'
