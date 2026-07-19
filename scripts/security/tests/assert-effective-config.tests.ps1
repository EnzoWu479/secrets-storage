$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$verifier = Join-Path $repoRoot "scripts/security/assert-effective-config.ps1"
$baseConfig = Join-Path $repoRoot "src-tauri/tauri.conf.json"
$overlayConfig = Join-Path $repoRoot "src-tauri/tauri.security-proof.conf.json"
$proofCapability = Join-Path $repoRoot "src-tauri/capabilities/security-proof.json"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("security-proof-config-" + [guid]::NewGuid().ToString("N"))
$powerShellExecutable = (Get-Process -Id $PID).Path
$script:passed = 0

function Write-JsonFile {
    param(
        [Parameter(Mandatory)]
        [string] $Path,
        [Parameter(Mandatory)]
        [object] $Value
    )

    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Path -Encoding utf8
}

function New-ArtifactTarget {
    param(
        [Parameter(Mandatory)]
        [string] $Name,
        [switch] $Proof,
        [switch] $MissingProofPermission,
        [switch] $ProofAclLeak
    )

    $target = Join-Path $tempRoot $Name
    $out = Join-Path $target "debug/build/secrets-storage-fixture/out"
    New-Item -ItemType Directory -Path $out -Force | Out-Null

    if ($Proof) {
        $permissions = [ordered]@{}
        $commands = @(
            "proof_authorized_probe",
            "proof_install_canary",
            "proof_lock",
            "proof_status"
        )

        foreach ($command in $commands) {
            if ($MissingProofPermission -and $command -eq "proof_status") {
                continue
            }

            $slug = $command.Replace("_", "-")
            $permissions["allow-$slug"] = @{
                identifier = "allow-$slug"
                commands = @{ allow = @($command); deny = @() }
            }
            $permissions["deny-$slug"] = @{
                identifier = "deny-$slug"
                commands = @{ allow = @(); deny = @($command) }
            }
        }

        Write-JsonFile -Path (Join-Path $out "capabilities.json") -Value @{
            "security-proof" = @{
                identifier = "security-proof"
                local = $true
                windows = @("security-proof")
                permissions = @(
                    "allow-proof-install-canary",
                    "allow-proof-authorized-probe",
                    "allow-proof-lock",
                    "allow-proof-status"
                )
            }
        }
        Write-JsonFile -Path (Join-Path $out "acl-manifests.json") -Value @{
            "__app-acl__" = @{ permissions = $permissions }
        }
    }
    else {
        Write-JsonFile -Path (Join-Path $out "capabilities.json") -Value @{
            default = @{
                identifier = "default"
                local = $true
                windows = @("main")
                permissions = @("core:app:allow-version")
            }
        }
        $normalAcl = @{ core = @{ permissions = @{} } }
        if ($ProofAclLeak) {
            $normalAcl["__app-acl__"] = @{
                permissions = @{
                    "allow-proof-status" = @{
                        identifier = "allow-proof-status"
                        commands = @{ allow = @("proof_status"); deny = @() }
                    }
                }
            }
        }
        Write-JsonFile -Path (Join-Path $out "acl-manifests.json") -Value $normalAcl
    }

    return $target
}

function Invoke-Case {
    param(
        [Parameter(Mandatory)]
        [string] $Name,
        [Parameter(Mandatory)]
        [int] $ExpectedExitCode,
        [Parameter(Mandatory)]
        [string[]] $Arguments
    )

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = @(& $powerShellExecutable -NoProfile -File $verifier @Arguments 2>&1)
        $actualExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($actualExitCode -ne $ExpectedExitCode) {
        throw "Case '$Name' expected exit $ExpectedExitCode, got $actualExitCode.`n$($output -join "`n")"
    }

    $script:passed++
    Write-Host "PASS $Name"
}

try {
    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
    $normalTarget = New-ArtifactTarget -Name "normal"
    $leakedNormalTarget = New-ArtifactTarget -Name "normal-leaked-acl" -ProofAclLeak
    $proofTarget = New-ArtifactTarget -Name "proof" -Proof
    $incompleteProofTarget = New-ArtifactTarget -Name "proof-incomplete" -Proof -MissingProofPermission

    $common = @(
        "-BaseConfigPath", $baseConfig,
        "-OverlayConfigPath", $overlayConfig,
        "-ProofCapabilityPath", $proofCapability
    )

    Invoke-Case -Name "normal config is isolated" -ExpectedExitCode 0 -Arguments (@(
        "-Mode", "Normal",
        "-TargetDir", $normalTarget,
        "-CargoFeatures", "default"
    ) + $common)

    Invoke-Case -Name "proof config is isolated" -ExpectedExitCode 0 -Arguments (@(
        "-Mode", "SecurityProof",
        "-TargetDir", $proofTarget,
        "-CargoFeatures", "security-proof"
    ) + $common)

    Invoke-Case -Name "normal rejects proof feature leakage" -ExpectedExitCode 1 -Arguments (@(
        "-Mode", "Normal",
        "-TargetDir", $normalTarget,
        "-CargoFeatures", "security-proof"
    ) + $common)

    Invoke-Case -Name "normal rejects proof ACL leakage" -ExpectedExitCode 1 -Arguments (@(
        "-Mode", "Normal",
        "-TargetDir", $leakedNormalTarget,
        "-CargoFeatures", "default"
    ) + $common)

    Invoke-Case -Name "proof rejects missing Cargo feature" -ExpectedExitCode 1 -Arguments (@(
        "-Mode", "SecurityProof",
        "-TargetDir", $proofTarget,
        "-CargoFeatures", "default"
    ) + $common)

    Invoke-Case -Name "proof rejects incomplete generated ACL" -ExpectedExitCode 1 -Arguments (@(
        "-Mode", "SecurityProof",
        "-TargetDir", $incompleteProofTarget,
        "-CargoFeatures", "security-proof"
    ) + $common)

    $duplicateOut = Join-Path $proofTarget "release/build/secrets-storage-other/out"
    New-Item -ItemType Directory -Path $duplicateOut -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $proofTarget "debug/build/secrets-storage-fixture/out/capabilities.json") -Destination $duplicateOut
    Copy-Item -LiteralPath (Join-Path $proofTarget "debug/build/secrets-storage-fixture/out/acl-manifests.json") -Destination $duplicateOut
    Invoke-Case -Name "proof rejects ambiguous build artifacts" -ExpectedExitCode 1 -Arguments (@(
        "-Mode", "SecurityProof",
        "-TargetDir", $proofTarget,
        "-CargoFeatures", "security-proof"
    ) + $common)

    $partialTarget = New-ArtifactTarget -Name "normal-partial-extra"
    $partialOut = Join-Path $partialTarget "release/build/secrets-storage-partial/out"
    New-Item -ItemType Directory -Path $partialOut -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $partialTarget "debug/build/secrets-storage-fixture/out/capabilities.json") -Destination $partialOut
    Invoke-Case -Name "normal rejects partial extra build artifacts" -ExpectedExitCode 1 -Arguments (@(
        "-Mode", "Normal",
        "-TargetDir", $partialTarget,
        "-CargoFeatures", "default"
    ) + $common)

    Write-Host "$script:passed security config verifier cases passed."
}
finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
