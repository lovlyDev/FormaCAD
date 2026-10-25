param(
    [Parameter(Mandatory = $true)]
    [string]$OcctRoot,
    [string]$TargetTriple = 'x86_64-pc-windows-msvc',
    [string]$RuntimeDirectory = ''
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

# Resolve once so worker and desktop Cargo invocations share the same location.
$targetBase = if ($env:CARGO_TARGET_DIR) {
    if ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) { $env:CARGO_TARGET_DIR }
    else { Join-Path $tauriRoot $env:CARGO_TARGET_DIR }
} else { Join-Path $tauriRoot 'target' }
$targetBase = [IO.Path]::GetFullPath($targetBase)
$env:CARGO_TARGET_DIR = $targetBase

. (Join-Path $PSScriptRoot 'windows-env.ps1')
$env:FORMA_OCCT_ROOT = $occt
$env:PATH = "$occtBin;$env:PATH"

Push-Location $tauriRoot
try {
    & cargo build --release --locked --target $TargetTriple --features native-occt --bin forma-cad-worker
    if ($LASTEXITCODE -ne 0) { throw 'Native CAD worker build failed.' }
} finally {
    Pop-Location
}

$worker = Join-Path $targetBase "$TargetTriple/release/forma-cad-worker.exe"
if (-not (Test-Path -LiteralPath $worker -PathType Leaf)) { throw 'Native CAD worker was not produced.' }

$binaryStage = Join-Path $tauriRoot 'binaries'
$libraryStage = Join-Path $tauriRoot 'resources/occt'
New-Item -ItemType Directory -Force -Path $binaryStage, $libraryStage | Out-Null
Copy-Item -LiteralPath $worker -Destination (Join-Path $binaryStage "forma-cad-worker-$TargetTriple.exe") -Force
$libraries = @(Get-ChildItem -LiteralPath $occtBin -Filter '*.dll' -File)
$extraLibraries = @(Get-ChildItem -LiteralPath $libraryStage -Filter '*.dll' -File |
    Where-Object { $_.Name -notin $libraries.Name })
if ($extraLibraries.Count -ne 0) {
    throw "Unverified extra OCCT DLLs in staging: $($extraLibraries.Name -join ', ')"
}

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

# Bundle the licensed Microsoft runtime locally for offline startup.
if (-not $RuntimeDirectory) {
    $redistRoot = Join-Path $env:VCINSTALLDIR 'Redist/MSVC'
    $candidates = @(Get-ChildItem -LiteralPath $redistRoot -Directory |
        Where-Object { $_.Name -match '^\d+\.\d+\.\d+$' } |
        Sort-Object { [version]$_.Name } -Descending)
    foreach ($candidate in $candidates) {
        foreach ($family in @('Microsoft.VC145.CRT', 'Microsoft.VC143.CRT')) {
            $path = Join-Path $candidate.FullName "x64/$family"
            if (Test-Path -LiteralPath (Join-Path $path 'msvcp140.dll') -PathType Leaf) {
                $RuntimeDirectory = $path
                break
            }
        }
        if ($RuntimeDirectory) { break }
    }
}
if (-not $RuntimeDirectory) { throw 'Visual Studio x64 redistributable runtime was not found.' }
& (Join-Path $PSScriptRoot 'stage-windows-runtime.ps1') `
    -SourceDirectory $RuntimeDirectory `
    -DestinationDirectory (Join-Path $tauriRoot 'resources/windows-runtime') `
    -MinimumVersion ([version]$env:VCToolsVersion)

# Freeze all native package inputs after staging, and recheck across bundling.
$stagedWorker = Join-Path $binaryStage "forma-cad-worker-$TargetTriple.exe"
if ((Get-FileHash -LiteralPath $stagedWorker).Hash -ne (Get-FileHash -LiteralPath $worker).Hash) {
    throw 'Staged native worker hash differs from the built worker.'
}
foreach ($library in $libraries) {
    if ((Get-FileHash -LiteralPath (Join-Path $libraryStage $library.Name)).Hash -ne
        (Get-FileHash -LiteralPath $library.FullName).Hash) {
        throw "Staged OCCT hash differs: $($library.Name)"
    }
}
$runtimeStage = Join-Path $tauriRoot 'resources/windows-runtime'
$inputFiles = @($stagedWorker) + @($libraries | ForEach-Object { Join-Path $libraryStage $_.Name }) +
    @(Get-ChildItem -LiteralPath $runtimeStage -Filter '*.dll' -File | ForEach-Object { $_.FullName })
$inputHashes = @{}
foreach ($path in $inputFiles) { $inputHashes[$path] = (Get-FileHash -LiteralPath $path).Hash }

Push-Location $workspace
try {
    & npm run tauri -w apps/desktop -- build --target $TargetTriple --features native-worker-client --bundles nsis --config (Join-Path $tauriRoot 'tauri.native.windows.conf.json')
    if ($LASTEXITCODE -ne 0) { throw 'Native Windows installer build failed.' }
} finally {
    Pop-Location
}
foreach ($path in $inputFiles) {
    if ((Get-FileHash -LiteralPath $path).Hash -ne $inputHashes[$path]) {
        throw "Native package input changed during bundling: $path"
    }
}
if (@(Get-ChildItem -LiteralPath $libraryStage -Filter '*.dll' -File).Count -ne $libraries.Count -or
    @(Get-ChildItem -LiteralPath $runtimeStage -Filter '*.dll' -File).Count -ne
    @($inputFiles | Where-Object { (Split-Path $_ -Parent) -eq $runtimeStage }).Count) {
    throw 'Native package input set changed during bundling.'
}
& (Join-Path $PSScriptRoot 'check-windows-host-imports.ps1') `
    -Executable (Join-Path $targetBase "$TargetTriple/release/forma-cad.exe")
Write-Host "Native installer: $(Join-Path $targetBase "$TargetTriple/release/bundle/nsis")"
