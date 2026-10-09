param(
    [Parameter(Mandatory = $true)]
    [string]$OcctRoot,
    [string]$TargetTriple = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$tauriRoot = Join-Path $workspace 'apps/desktop/src-tauri'
$occt = (Resolve-Path -LiteralPath $OcctRoot).Path
$occtBin = Join-Path $occt 'win64/vc14/bin'
$occtLib = Join-Path $occt 'win64/vc14/lib'
$occtInclude = Join-Path $occt 'inc'
if (-not (Test-Path -LiteralPath $occtBin -PathType Container) -or
    -not (Test-Path -LiteralPath $occtLib -PathType Container) -or
    -not (Test-Path -LiteralPath $occtInclude -PathType Container)) {
    throw 'OCCT root must contain win64/vc14/bin, win64/vc14/lib, and inc.'
}
if ($TargetTriple -ne 'x86_64-pc-windows-msvc') {
    throw 'This packaging script currently supports Windows x64 only.'
}

. (Join-Path $PSScriptRoot 'windows-env.ps1')
$env:FORMA_OCCT_ROOT = $occt
$env:PATH = "$occtBin;$env:PATH"

Push-Location $tauriRoot
try {
    & cargo build --release --target $TargetTriple --features native-occt --bin forma-cad-worker
    if ($LASTEXITCODE -ne 0) { throw 'Native CAD worker build failed.' }
} finally {
    Pop-Location
}

$targetBase = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $tauriRoot 'target' }
$worker = Join-Path $targetBase "$TargetTriple/release/forma-cad-worker.exe"
if (-not (Test-Path -LiteralPath $worker -PathType Leaf)) { throw 'Native CAD worker was not produced.' }

$binaryStage = Join-Path $tauriRoot 'binaries'
$libraryStage = Join-Path $tauriRoot 'resources/occt'
New-Item -ItemType Directory -Force -Path $binaryStage, $libraryStage | Out-Null
Copy-Item -LiteralPath $worker -Destination (Join-Path $binaryStage "forma-cad-worker-$TargetTriple.exe") -Force
$libraries = @(Get-ChildItem -LiteralPath $occtBin -Filter '*.dll' -File)
if ($libraries.Count -eq 0) { throw 'No OCCT runtime libraries were found.' }
foreach ($library in $libraries) {
    Copy-Item -LiteralPath $library.FullName -Destination (Join-Path $libraryStage $library.Name) -Force
}
$sourceRoot = Join-Path (Split-Path $occt -Parent) 'occt-source'
foreach ($license in @('LICENSE_LGPL_21.txt', 'OCCT_LGPL_EXCEPTION.txt')) {
    $source = Join-Path $sourceRoot $license
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "OCCT license file is missing: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $libraryStage $license) -Force
}

Push-Location $workspace
try {
    & npm run tauri -w apps/desktop -- build --target $TargetTriple --features native-occt --bundles nsis --config (Join-Path $tauriRoot 'tauri.native.windows.conf.json')
    if ($LASTEXITCODE -ne 0) { throw 'Native Windows installer build failed.' }
} finally {
    Pop-Location
}
Write-Host "Native installer: $(Join-Path $targetBase "$TargetTriple/release/bundle/nsis")"
