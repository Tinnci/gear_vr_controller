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

function Invoke-OptionalTool {
    param(
        [Parameter(Mandatory = $true)][string]$Tool,
        [Parameter(Mandatory = $true)][string]$Description,
        [Parameter(Mandatory = $true)][scriptblock]$Command
    )

    if (Test-Tool $Tool) {
        & $Command
    } else {
        Write-Warning "Skipping ${Description}: ${Tool} is not installed."
    }
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
        Invoke-OptionalTool "cargo-llvm-cov" "coverage" {
            $coverageArgs = @("llvm-cov", "--workspace", "--all-features")
            if ($FailUnderLines -gt 0) {
                $coverageArgs += @("--fail-under-lines", $FailUnderLines)
            } else {
                $coverageArgs += "--summary-only"
            }
            Invoke-Native "cargo" $coverageArgs
        }
    }

    Invoke-Step "Dependency policy" {
        Invoke-OptionalTool "cargo-deny" "dependency policy" {
            Invoke-Native "cargo" @("deny", "check")
        }
    }

    Invoke-Step "Unused dependencies" {
        Invoke-OptionalTool "cargo-machete" "unused dependency scan" {
            Invoke-Native "cargo" @("machete")
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

    Invoke-Step "Module structure" {
        Invoke-OptionalTool "cargo-modules" "module structure report" {
            Invoke-Native "cargo" @(
                "modules",
                "structure",
                "--bin",
                "gear_vr_controller_rust",
                "--no-fns",
                "--focus-on",
                "gear_vr_controller_rust::infrastructure::bluetooth",
                "--max-depth",
                "4"
            )
        }
    }

    Invoke-Step "Duplication scan" {
        Invoke-OptionalTool "jscpd" "duplication scan" {
            Invoke-Native "jscpd" @(
                "src",
                "--threshold",
                "5",
                "--min-lines",
                "8",
                "--min-tokens",
                "80"
            )
        }
    }
}
