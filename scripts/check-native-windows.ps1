param(
    [Parameter(Mandatory = $true)]
    [string]$WorkerPath,
    [Parameter(Mandatory = $true)]
    [string]$LibraryDirectory
)

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$worker = (Resolve-Path -LiteralPath $WorkerPath).Path
$libraries = (Resolve-Path -LiteralPath $LibraryDirectory).Path
$smokeDirectory = Join-Path $workspace ".local/native-smoke-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $smokeDirectory -Force | Out-Null
$smokeWorker = Join-Path $smokeDirectory 'forma-cad-worker.exe'
Copy-Item -LiteralPath $worker -Destination $smokeWorker
$dlls = @(Get-ChildItem -LiteralPath $libraries -Filter '*.dll' -File)
if ($dlls.Count -eq 0) { throw 'No bundled OCCT DLLs were found.' }
foreach ($dll in $dlls) { Copy-Item -LiteralPath $dll.FullName -Destination (Join-Path $smokeDirectory $dll.Name) }

$request = @'
{"protocolVersion":1,"requestId":"packaged_worker_smoke","bodyId":"body","document":{"schemaVersion":2,"revisionId":"revision_smoke","parameters":[],"features":[{"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":10},"depth":{"kind":"literal","mm":20}}},{"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":5}}}],"bodies":[{"id":"body","name":"Plate","sourceFeatureId":"pad"}]}}
'@
$start = [System.Diagnostics.ProcessStartInfo]::new($smokeWorker)
$start.WorkingDirectory = $smokeDirectory
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$process = [System.Diagnostics.Process]::Start($start)
try {
    $process.StandardInput.Write($request)
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(30000)) {
        $process.Kill($true)
        throw 'Packaged CAD worker timed out.'
    }
    $stderr = $process.StandardError.ReadToEnd()
    if ($process.ExitCode -ne 0) { throw "Packaged CAD worker failed: $stderr" }
} finally {
    $process.Dispose()
}

$result = Get-Content -LiteralPath (Join-Path $smokeDirectory 'result.json') -Raw | ConvertFrom-Json
if ($result.status -ne 'completed' -or $result.requestId -ne 'packaged_worker_smoke') {
    throw 'Packaged CAD worker returned an invalid response.'
}
if ([math]::Abs([double]$result.volumeMm3 - 1000.0) -gt 0.001) {
    throw 'Packaged CAD worker produced incorrect volume.'
}
$step = Join-Path $smokeDirectory 'model.step'
$glb = Join-Path $smokeDirectory 'preview.glb'
if ((Get-FileHash -LiteralPath $step -Algorithm SHA256).Hash.ToLowerInvariant() -ne $result.stepSha256 -or
    (Get-FileHash -LiteralPath $glb -Algorithm SHA256).Hash.ToLowerInvariant() -ne $result.previewSha256) {
    throw 'Packaged CAD worker output checksum mismatch.'
}
$header = [System.IO.File]::ReadAllBytes($glb)
if ($header.Length -lt 12 -or [System.Text.Encoding]::ASCII.GetString($header, 0, 4) -ne 'glTF') {
    throw 'Packaged CAD worker did not produce a GLB file.'
}
Write-Host "Packaged CAD worker passed: $smokeDirectory"
