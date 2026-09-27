# Validate the NuGet payloads downloaded outside Cargo.lock, then extract fresh caches.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$runtimeLock = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'runtime-lock.json') -Raw | ConvertFrom-Json
$runtimeCache = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'windows-reactor-setup/temp'))
$null = New-Item -ItemType Directory -Path $runtimeCache -Force
foreach ($package in $runtimeLock.packages) {
    $archive = Join-Path $runtimeCache "$($package.name).$($package.version).nupkg"
    if (!(Test-Path -LiteralPath $archive)) {
        Invoke-WebRequest -Uri "https://www.nuget.org/api/v2/package/$($package.name)/$($package.version)" -OutFile $archive
    }
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $package.sha256) {
        throw "Runtime package checksum mismatch: $($package.name) $($package.version)"
    }
    $extract = [IO.Path]::GetFullPath((Join-Path $runtimeCache "$($package.name)-$($package.version)"))
    # Delete only these exact, derived cache directories after validating containment.
    if ([IO.Path]::GetDirectoryName($extract) -ne $runtimeCache) { throw 'Unsafe runtime cache path' }
    if (Test-Path -LiteralPath $extract) { Remove-Item -LiteralPath $extract -Recurse -Force }
    $null = New-Item -ItemType Directory -Path $extract
    & "$env:SystemRoot/System32/tar.exe" -xf $archive -C $extract --strip-components=1
    if ($LASTEXITCODE -ne 0) { throw "Cannot extract $archive" }
    $licenses = Join-Path $extract 'licenses'
    $null = New-Item -ItemType Directory -Path $licenses -Force
    $licenseEntries = @(& "$env:SystemRoot/System32/tar.exe" -tf $archive | Where-Object { $_ -match '^(license|notice)\.txt$' })
    if ($LASTEXITCODE -ne 0 -or $licenseEntries.Count -eq 0) { throw "Missing license records for $($package.name)" }
    & "$env:SystemRoot/System32/tar.exe" -xf $archive -C $licenses @licenseEntries
    if ($LASTEXITCODE -ne 0) { throw "Cannot extract license records for $($package.name)" }
    Write-Host "Verified $($package.name) $($package.version)"
}
$sdk = Join-Path $runtimeCache 'Microsoft.WindowsAppSDK.Runtime-2.4.0'
$msix = Join-Path $sdk 'MSIX/win10-x64/Microsoft.WindowsAppRuntime.2.msix'
$msixExtract = Join-Path $sdk '.msix_extract'
$null = New-Item -ItemType Directory -Path $msixExtract -Force
& "$env:SystemRoot/System32/tar.exe" -xf $msix -C $msixExtract
if ($LASTEXITCODE -ne 0) { throw 'Cannot extract Windows App Runtime MSIX' }
