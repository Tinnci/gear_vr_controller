param([Parameter(Mandatory=$true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$Executable = [IO.Path]::GetFullPath($Executable)
$previousLogFilter = $env:RUST_LOG
$smokeStartedUtc = [DateTime]::UtcNow
try {
    # The smoke child must emit lifecycle records even if the caller filters INFO out.
    $env:RUST_LOG = 'info'
    $process = Start-Process -FilePath $Executable -ArgumentList '--smoke-test' -WorkingDirectory ([IO.Path]::GetDirectoryName($Executable)) -WindowStyle Hidden -PassThru
} finally {
    $env:RUST_LOG = $previousLogFilter
}
try {
    if (!$process.WaitForExit(20000)) { $process.Kill(); throw 'Native app smoke test timed out' }
    if ($process.ExitCode -ne 0) { throw "Native app smoke test failed: $($process.ExitCode)" }
    $logDir = Join-Path ([IO.Path]::GetTempPath()) 'GearVRController-smoke/logs'
    $files = @(Get-ChildItem -LiteralPath $logDir -File -Filter "gear_vr_controller-*-*-$($process.Id)-*.log" |
        Where-Object { $_.LastWriteTimeUtc -ge $smokeStartedUtc })
    $records = @($files | ForEach-Object { Get-Content -LiteralPath $_.FullName } | ForEach-Object { $_ | ConvertFrom-Json })
    if (!($records | Where-Object { $_.fields.event -eq 'app.started' -and $_.fields.pid -eq $process.Id })) {
        throw 'Native logger did not record application startup'
    }
    $stopped = $records | Where-Object { $_.fields.event -eq 'app.stopped' } | Select-Object -Last 1
    if (!$stopped -or $stopped.fields.dropped_events -ne 0 -or $stopped.fields.oversized_events -ne 0 -or
        $stopped.fields.io_errors -ne 0 -or $stopped.fields.cleanup_errors -ne 0) {
        throw 'Native logger did not drain cleanly at shutdown'
    }
    Write-Host 'Native window initialization and graceful worker shutdown passed.'
    Write-Host 'Structured lifecycle logs and background writer shutdown passed.'
} finally { $process.Dispose() }
