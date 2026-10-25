param(
    [Parameter(Mandatory = $true)][string]$SourceDirectory,
    [Parameter(Mandatory = $true)][string]$DestinationDirectory,
    [version]$MinimumVersion = [version]'14.0'
)

# Use the licensed Visual Studio x64 redistributable directory, never System32.
$ErrorActionPreference = 'Stop'
$source = (Resolve-Path -LiteralPath $SourceDirectory).Path
if ((Split-Path $source -Leaf) -notin @('Microsoft.VC143.CRT', 'Microsoft.VC145.CRT') -or
    (Split-Path (Split-Path $source -Parent) -Leaf) -ne 'x64') {
    throw 'Expected a Visual Studio x64/Microsoft.VC143.CRT or Microsoft.VC145.CRT redistributable directory.'
}
$required = @('msvcp140.dll', 'msvcp140_2.dll', 'vcruntime140.dll', 'vcruntime140_1.dll')
$files = @(Get-ChildItem -LiteralPath $source -Filter '*.dll' -File)
foreach ($name in $required) {
    if ($name -notin $files.Name) { throw "Required redistributable is absent: $name" }
}
# Verify every input before staging any file. Copies remain signed by Microsoft.
foreach ($file in $files) {
    $fileVersion = [version]::new($file.VersionInfo.FileMajorPart, $file.VersionInfo.FileMinorPart,
        $file.VersionInfo.FileBuildPart, $file.VersionInfo.FilePrivatePart)
    if ($fileVersion -lt $MinimumVersion) {
        throw "Redistributable is older than the build toolchain: $($file.Name) ($fileVersion < $MinimumVersion)"
    }
    if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "Redistributable must not be a link: $($file.Name)"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    if ($signature.Status -ne 'Valid' -or
        $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation(?:,|$)') {
        throw "Microsoft signature verification failed: $($file.Name)"
    }
    $stream = [IO.File]::OpenRead($file.FullName)
    try {
        $reader = [IO.BinaryReader]::new($stream)
        if ($reader.ReadUInt16() -ne 0x5a4d) { throw 'Invalid redistributable PE header.' }
        $stream.Position = 0x3c
        $peOffset = $reader.ReadUInt32()
        if ($peOffset -gt $stream.Length - 6) { throw 'Invalid redistributable PE offset.' }
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne 0x8664) {
            throw "Redistributable must be Windows x64: $($file.Name)"
        }
    } finally { $stream.Dispose() }
}
New-Item -ItemType Directory -Force -Path $DestinationDirectory | Out-Null
$extras = @(Get-ChildItem -LiteralPath $DestinationDirectory -Filter '*.dll' -File |
    Where-Object { $_.Name -notin $files.Name })
if ($extras.Count -ne 0) {
    throw "Unverified extra runtime DLLs in staging: $($extras.Name -join ', ')"
}
foreach ($file in $files) {
    $expected = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    $target = Join-Path $DestinationDirectory $file.Name
    if (Test-Path -LiteralPath $target) {
        if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ne $expected) {
            throw "Staged runtime differs; review it before replacement: $($file.Name)"
        }
    } else {
        Copy-Item -LiteralPath $file.FullName -Destination $target
    }
    if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ne $expected) {
        throw "Runtime copy verification failed: $($file.Name)"
    }
}
