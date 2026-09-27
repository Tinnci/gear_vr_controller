param([string]$Executable)
$ErrorActionPreference = 'Stop'
if (!$Executable) {
    $Executable = Join-Path $PSScriptRoot 'gear_vr_controller_rust.exe'
    if (!(Test-Path -LiteralPath $Executable -PathType Leaf)) {
        $Executable = Join-Path $PSScriptRoot '../dist/gear_vr_controller-0.1.0-win-x64/gear_vr_controller_rust.exe'
    }
}
$Executable = [IO.Path]::GetFullPath($Executable)
if (!(Test-Path -LiteralPath $Executable -PathType Leaf)) { throw "Executable not found: $Executable" }
$previousFilter = $env:RUST_LOG
try {
    # Only the child inherits this filter. Do not change saved settings or the user environment.
    $env:RUST_LOG = 'info,gear_vr_controller_rust::infrastructure::bluetooth=debug'
    $process = Start-Process -FilePath $Executable -WorkingDirectory ([IO.Path]::GetDirectoryName($Executable)) -WindowStyle Hidden -PassThru
    Write-Host "Diagnostic app started (PID $($process.Id)). Reproduce the connection failure, then close the app."
    Write-Host 'Logs use your configured folder; the default is %LOCALAPPDATA%/GearVRController/logs.'
} finally {
    $env:RUST_LOG = $previousFilter
}
