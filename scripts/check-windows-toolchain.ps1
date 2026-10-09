$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-env.ps1')

$sdkLibrary = Join-Path $env:WindowsSdkDir "Lib\$env:WindowsSDKVersion\um\x64\kernel32.lib"
if (-not (Test-Path -LiteralPath $sdkLibrary -PathType Leaf)) {
    throw "Windows SDK library is missing: $sdkLibrary"
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$probeDirectory = Join-Path $workspace ".local/toolchain-probe-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $probeDirectory -Force | Out-Null
$source = Join-Path $probeDirectory 'main.rs'
$binary = Join-Path $probeDirectory 'forma-link-probe.exe'
[System.IO.File]::WriteAllText($source, 'fn main() {}')
& rustc --crate-name forma_link_probe --target x86_64-pc-windows-msvc -o $binary $source
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw 'Rust cannot link against the selected Windows SDK.'
}
Write-Host "Rust linker passed with Windows SDK $env:WindowsSDKVersion"
