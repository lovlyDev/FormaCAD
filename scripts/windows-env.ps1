# Select a complete C++ toolchain when multiple Visual Studio editions are installed.
$ErrorActionPreference = 'Stop'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (!(Test-Path -LiteralPath $vswhere)) { throw 'Install Visual Studio Build Tools with Desktop development with C++.' }
$installations = & $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$selectedTools = $null
foreach ($installation in $installations) {
    $toolsRoot = Join-Path $installation 'VC\Tools\MSVC'
    foreach ($tools in (Get-ChildItem -LiteralPath $toolsRoot -Directory | Sort-Object Name -Descending)) {
        if ((Test-Path -LiteralPath (Join-Path $tools.FullName 'include\excpt.h')) -and (Test-Path -LiteralPath (Join-Path $tools.FullName 'lib\x64\msvcrt.lib'))) {
            $selectedTools = $tools.FullName
            $env:VCINSTALLDIR = Join-Path $installation 'VC\'
            break
        }
    }
    if ($selectedTools) { break }
}
if (!$selectedTools) { throw 'No complete x64 C++ toolchain found.' }
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
$sdkVersion = (Get-ChildItem -LiteralPath (Join-Path $sdkRoot 'Lib') -Directory | Sort-Object Name -Descending | Select-Object -First 1).Name
$env:LIB = "$selectedTools\lib\x64;$sdkRoot\Lib\$sdkVersion\ucrt\x64;$sdkRoot\Lib\$sdkVersion\um\x64"
$env:INCLUDE = "$selectedTools\include;$sdkRoot\Include\$sdkVersion\ucrt;$sdkRoot\Include\$sdkVersion\shared;$sdkRoot\Include\$sdkVersion\um;$sdkRoot\Include\$sdkVersion\winrt"
$env:PATH = "$selectedTools\bin\Hostx64\x64;$sdkRoot\bin\$sdkVersion\x64;$env:PATH"
$env:VCToolsInstallDir = "$selectedTools\"
$env:VCToolsVersion = Split-Path $selectedTools -Leaf
$env:WindowsSdkDir = "$sdkRoot\"
$env:WindowsSDKVersion = "$sdkVersion\"
$env:CC = Join-Path $selectedTools 'bin\Hostx64\x64\cl.exe'
$env:CXX = $env:CC
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = Join-Path $selectedTools 'bin\Hostx64\x64\link.exe'
Write-Host "Using C++ toolchain: $selectedTools"
