param([Parameter(Mandatory = $true)][string]$Executable)

# Inspect the actual executable; source cfgs alone do not establish isolation.
$ErrorActionPreference = 'Stop'
$path = (Resolve-Path -LiteralPath $Executable).Path
$output = & dumpbin /nologo /dependents $path 2>&1
if ($LASTEXITCODE -ne 0) { throw 'Desktop PE dependency inspection failed.' }
$imports = @($output | ForEach-Object {
    if ($_ -match '^\s+([a-zA-Z0-9_.-]+\.dll)\s*$') { $matches[1] }
})
if ($imports.Count -eq 0) { throw 'Desktop PE dependency list is empty or unreadable.' }
$kernelImports = @($imports | Where-Object { $_ -match '^TK.*\.dll$' })
if ($kernelImports.Count -ne 0) {
    throw "Desktop directly imports OCCT: $($kernelImports -join ', ')"
}
# Package-specific third-party dependencies must be reviewed recursively, rather
# than silently accepted as system DLLs. Current pure host uses this system set.
$systemImports = @(
    'kernel32.dll', 'advapi32.dll', 'oleaut32.dll', 'bcryptprimitives.dll',
    'ntdll.dll', 'user32.dll', 'ole32.dll', 'shlwapi.dll', 'comctl32.dll',
    'gdi32.dll', 'dwmapi.dll', 'shell32.dll', 'ws2_32.dll', 'crypt32.dll',
    'bcrypt.dll', 'msvcrt.dll', 'ucrtbase.dll', 'secur32.dll', 'version.dll',
    # Tauri 2.12: actual host imports COM allocation/marshalling and IME APIs.
    # Both were checked against signed Microsoft DLLs in Windows System32.
    'combase.dll', 'imm32.dll'
)
$unreviewed = @($imports | Where-Object {
    $_ -notin $systemImports -and $_ -notmatch '^api-ms-win-[a-z0-9-]+\.dll$'
})
if ($unreviewed.Count -ne 0) {
    throw "Review desktop dependency closure before packaging: $($unreviewed -join ', ')"
}
[pscustomobject]@{
    executable = $path
    sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    imports = $imports
    occtImports = $kernelImports
} | ConvertTo-Json -Depth 3
