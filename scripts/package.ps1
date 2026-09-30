param([string]$Target = 'x86_64-pc-windows-msvc')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $repo
try {
    if ($Target -ne 'x86_64-pc-windows-msvc') { throw 'Only the verified x64 release target is supported' }
    & (Join-Path $PSScriptRoot 'prepare-runtime.ps1')
    cargo build --release --locked --target $Target
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    $version = (cargo metadata --no-deps --format-version 1 --locked | ConvertFrom-Json).packages[0].version
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read package metadata' }
    $release = Join-Path $repo "target/$Target/release"
    $dist = Join-Path $repo 'dist'
    $packageDir = [IO.Path]::GetFullPath((Join-Path $dist "gear_vr_controller-$version-win-x64"))
    if ([IO.Path]::GetDirectoryName($packageDir) -ne [IO.Path]::GetFullPath($dist)) { throw 'Unsafe package path' }
    if (Test-Path -LiteralPath $packageDir) { Remove-Item -LiteralPath $packageDir -Recurse -Force }
    $null = New-Item -ItemType Directory -Path $packageDir -Force
    Copy-Item -LiteralPath (Join-Path $release 'gear_vr_controller_rust.exe') -Destination $packageDir
    Copy-Item -LiteralPath (Join-Path $release 'gearvr-debug.exe') -Destination $packageDir
    Copy-Item -LiteralPath (Join-Path $repo 'docs/DEBUG_CLI.md') -Destination $packageDir
    Copy-Item -LiteralPath (Join-Path $repo 'assets/app-icon.ico') -Destination $packageDir
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'start-diagnostics.ps1') -Destination $packageDir
    $runtimeRoot = Join-Path $env:LOCALAPPDATA 'windows-reactor-setup/temp/Microsoft.WindowsAppSDK.Runtime-2.4.0/.msix_extract'
    foreach ($name in (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'runtime-files.txt') | Where-Object { $_.Trim() })) {
        $source = Join-Path $runtimeRoot $name
        if (Test-Path -LiteralPath $source) {
            # Package from the verified extracted payload; preserve its relative layout.
            Copy-Item -LiteralPath $source -Destination $packageDir -Recurse -Force
        }
    }
    $webview = Join-Path $env:LOCALAPPDATA 'windows-reactor-setup/temp/Microsoft.Web.WebView2-1.0.4078.44/win-x64/native_uap/Microsoft.Web.WebView2.Core.dll'
    Copy-Item -LiteralPath $webview -Destination $packageDir
    foreach ($required in @('Microsoft.UI.Xaml.dll', 'Microsoft.WindowsAppRuntime.dll', 'resources.pri', 'Microsoft.UI.Xaml', 'en-us', 'Microsoft.Web.WebView2.Core.dll')) {
        if (!(Test-Path -LiteralPath (Join-Path $packageDir $required))) { throw "Missing runtime payload: $required" }
    }
    Copy-Item -LiteralPath (Join-Path $repo 'LICENSE'), (Join-Path $repo 'README.md'), (Join-Path $repo 'THIRD_PARTY_NOTICES.md'), (Join-Path $PSScriptRoot 'runtime-lock.json') -Destination $packageDir
    $noticeDir = Join-Path $packageDir 'licenses'
    $null = New-Item -ItemType Directory -Path $noticeDir -Force
    foreach ($name in @('Microsoft.WindowsAppSDK.Runtime-2.4.0', 'Microsoft.Web.WebView2-1.0.4078.44')) {
        $destination = Join-Path $noticeDir $name
        $null = New-Item -ItemType Directory -Path $destination -Force
        Copy-Item -Path (Join-Path $env:LOCALAPPDATA "windows-reactor-setup/temp/$name/licenses/*.txt") -Destination $destination
    }
    $manifest = Get-ChildItem -LiteralPath $packageDir -File -Recurse | ForEach-Object {
        $relative = [IO.Path]::GetRelativePath($packageDir, $_.FullName).Replace('\', '/')
        "$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant())  $relative"
    }
    $manifest | Set-Content -LiteralPath (Join-Path $packageDir 'SHA256SUMS') -Encoding utf8
    & (Join-Path $PSScriptRoot 'smoke.ps1') -Executable (Join-Path $packageDir 'gear_vr_controller_rust.exe')
    & (Join-Path $PSScriptRoot 'smoke-cli.ps1') -Executable (Join-Path $packageDir 'gearvr-debug.exe')
    $zip = "$packageDir.zip"
    if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip }
    Compress-Archive -LiteralPath $packageDir -DestinationPath $zip
    "$((Get-FileHash -LiteralPath $zip).Hash.ToLowerInvariant())  $([IO.Path]::GetFileName($zip))" | Set-Content -LiteralPath "$zip.sha256" -Encoding utf8
    # Symbols are a separate maintainer artifact, not part of the user package.
    Copy-Item -LiteralPath (Join-Path $release 'gear_vr_controller_rust.pdb') -Destination $dist -Force
    Copy-Item -LiteralPath (Join-Path $release 'gearvr_debug.pdb') -Destination $dist -Force
    Write-Host "Package: $zip"
} finally { Pop-Location }
