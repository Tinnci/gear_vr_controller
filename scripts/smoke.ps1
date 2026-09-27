param([Parameter(Mandatory=$true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$Executable = [IO.Path]::GetFullPath($Executable)
$process = Start-Process -FilePath $Executable -ArgumentList '--smoke-test' -WorkingDirectory ([IO.Path]::GetDirectoryName($Executable)) -WindowStyle Hidden -PassThru
try {
    if (!$process.WaitForExit(20000)) { $process.Kill(); throw 'Native app smoke test timed out' }
    if ($process.ExitCode -ne 0) { throw "Native app smoke test failed: $($process.ExitCode)" }
    Write-Host 'Native window initialization and graceful worker shutdown passed.'
} finally { $process.Dispose() }
