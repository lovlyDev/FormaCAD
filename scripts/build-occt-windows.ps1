param(
    [string]$SourceRoot = '.local\occt-source',
    [string]$BuildRoot = '.local\occt-build',
    [int]$Jobs = 4
)

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$source = Join-Path $workspace $SourceRoot
$build = Join-Path $workspace $BuildRoot

if (-not (Test-Path -LiteralPath (Join-Path $source 'CMakeLists.txt') -PathType Leaf)) {
    New-Item -ItemType Directory -Path (Split-Path $source -Parent) -Force | Out-Null
    & git clone --depth 1 --branch V8_0_1 https://github.com/Open-Cascade-SAS/OCCT.git $source
    if ($LASTEXITCODE -ne 0) { throw 'OCCT 8.0.1 checkout failed.' }
}

if (-not (Test-Path -LiteralPath (Join-Path $build 'win64/vc14/lib/TKernel.lib') -PathType Leaf)) {
    & cmake -S $source -B $build -G 'Visual Studio 17 2022' -A x64 `
        -DBUILD_LIBRARY_TYPE=Shared `
        -DBUILD_MODULE_Draw=OFF `
        -DBUILD_MODULE_Visualization=OFF `
        -DUSE_TK=OFF `
        -DUSE_FREETYPE=OFF `
        -DUSE_TBB=OFF
    if ($LASTEXITCODE -ne 0) { throw 'OCCT 8.0.1 configuration failed.' }
    & cmake --build $build --config Release --parallel $Jobs
    if ($LASTEXITCODE -ne 0) { throw 'OCCT 8.0.1 build failed.' }
}

Write-Host "OCCT 8.0.1 ready: $build"
