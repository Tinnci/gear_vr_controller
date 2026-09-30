param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-Records {
    param([string[]]$Lines, [string]$FinalEvent)
    $records = @($Lines | ForEach-Object { $_ | ConvertFrom-Json })
    if ($records.Count -eq 0 -or $records[-1].event -ne $FinalEvent) { throw 'CLI final event is missing' }
    foreach ($record in $records) {
        if ($record.schema -ne 1 -or $record.pid -le 0 -or $record.elapsed_ms -lt 0) { throw 'Invalid CLI envelope' }
    }
    return $records
}

# These commands do not initialize WinRT, open hardware or write preferences/logs.
$helpLines = @(& $Executable --help)
if ($LASTEXITCODE -ne 0) { throw 'CLI help failed' }
$records = Assert-Records $helpLines 'completed'
if ($records[0].event -ne 'help' -or !$records[-1].data.success) { throw 'CLI help contract failed' }
$failureLines = @(& $Executable connect)
if ($LASTEXITCODE -eq 0) { throw 'CLI accepted a connection without an explicit address' }
$records = Assert-Records $failureLines 'failed'
if ($records[-1].data.success) { throw 'CLI failure contract failed' }
Write-Host 'CLI smoke passed: JSON envelope, help and target validation'
# The deliberately rejected command is not the script's exit status.
$global:LASTEXITCODE = 0
