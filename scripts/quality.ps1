param(
    [switch]$Full,
    [int]$FailUnderLines = 0
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Command
    )

    Write-Host ""
    Write-Host "== $Name ==" -ForegroundColor Cyan
    & $Command
}

function Test-Tool {
    param([Parameter(Mandatory = $true)][string]$Name)

    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Measure-RustSource {
    $files = Get-ChildItem -Path src -Filter *.rs -Recurse
    $metrics = foreach ($file in $files) {
        $lines = @(Get-Content -LiteralPath $file.FullName)
        $blank = @($lines | Where-Object { $_.Trim().Length -eq 0 }).Count
        $comments = @($lines | Where-Object {
            $trimmed = $_.TrimStart()
            $trimmed.StartsWith("//") -or
                $trimmed.StartsWith("/*") -or
                $trimmed.StartsWith("*") -or
                $trimmed.StartsWith("*/")
        }).Count
        $code = $lines.Count - $blank - $comments
        $branchTokens = @(
            $lines | Select-String -Pattern "\b(if|else|match|for|while|loop)\b"
        ).Count
        $commentPercent = if (($code + $comments) -eq 0) {
            0
        } else {
            [math]::Round(($comments * 100.0) / ($code + $comments), 1)
        }

        [pscustomobject]@{
            Path = $file.FullName.Replace((Get-Location).Path + "\", "")
            Lines = $lines.Count
            Code = $code
            Comments = $comments
            CommentPercent = $commentPercent
            BranchTokens = $branchTokens
        }
    }

    $metrics |
        Sort-Object -Property BranchTokens, Lines -Descending |
        Select-Object -First 12 |
        Format-Table -AutoSize
}

Invoke-Step "Format" {
    Invoke-Native "cargo" @("fmt", "--all", "--", "--check")
}

Invoke-Step "Check" {
    Invoke-Native "cargo" @("check", "--workspace", "--all-targets", "--all-features", "--locked")
}

Invoke-Step "Clippy" {
    Invoke-Native "cargo" @(
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings"
    )
}

Invoke-Step "Test" {
    Invoke-Native "cargo" @("test", "--workspace", "--all-targets", "--all-features")
}

if ($Full) {
    Invoke-Step "Release build" {
        Invoke-Native "cargo" @("build", "--release", "--locked")
    }

    Invoke-Step "Coverage" {
        if (Test-Tool "cargo-llvm-cov") {
            $coverageArgs = @("llvm-cov", "--workspace", "--all-features")
            if ($FailUnderLines -gt 0) {
                $coverageArgs += @("--fail-under-lines", $FailUnderLines)
            } else {
                $coverageArgs += "--summary-only"
            }
            Invoke-Native "cargo" $coverageArgs
        } else {
            Write-Warning "Skipping coverage: cargo-llvm-cov is not installed."
        }
    }

    Invoke-Step "Dependency policy" {
        if (Test-Tool "cargo-deny") {
            Invoke-Native "cargo" @("deny", "check")
        } else {
            Write-Warning "Skipping dependency policy: cargo-deny is not installed."
        }
    }

    Invoke-Step "Unused dependencies" {
        if (Test-Tool "cargo-machete") {
            Invoke-Native "cargo" @("machete")
        } else {
            Write-Warning "Skipping unused dependency scan: cargo-machete is not installed."
        }
    }

    Invoke-Step "Source metrics" {
        if (Test-Tool "tokei") {
            Invoke-Native "tokei" @("src")
        } else {
            Write-Warning "tokei is not installed; using built-in Rust source metrics."
            Measure-RustSource
        }
    }

    Invoke-Step "Duplication scan" {
        if (Test-Tool "jscpd") {
            Invoke-Native "jscpd" @("src", "--threshold", "5", "--min-lines", "8", "--min-tokens", "80")
        } else {
            Write-Warning "Skipping duplication scan: jscpd is not installed."
        }
    }
}
