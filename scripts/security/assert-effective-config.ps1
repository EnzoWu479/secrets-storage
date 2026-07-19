[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet("Normal", "SecurityProof")]
    [string] $Mode,

    [Parameter(Mandatory)]
    [string] $TargetDir,

    [Parameter(Mandatory)]
    [string] $CargoFeatures,

    [string] $BaseConfigPath,
    [string] $OverlayConfigPath,
    [string] $ProofCapabilityPath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($BaseConfigPath)) {
    $BaseConfigPath = Join-Path $PSScriptRoot "../../src-tauri/tauri.conf.json"
}
if ([string]::IsNullOrWhiteSpace($OverlayConfigPath)) {
    $OverlayConfigPath = Join-Path $PSScriptRoot "../../src-tauri/tauri.security-proof.conf.json"
}
if ([string]::IsNullOrWhiteSpace($ProofCapabilityPath)) {
    $ProofCapabilityPath = Join-Path $PSScriptRoot "../../src-tauri/capabilities/security-proof.json"
}

function Fail {
    param([Parameter(Mandatory)][string] $Message)
    throw "effective config verification failed: $Message"
}

function Read-Json {
    param([Parameter(Mandatory)][string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "required JSON file does not exist: $Path"
    }

    try {
        return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    }
    catch {
        Fail "cannot parse JSON file '$Path': $($_.Exception.Message)"
    }
}

function Get-Property {
    param(
        [AllowNull()][object] $Object,
        [Parameter(Mandatory)][string] $Name
    )

    if ($null -eq $Object) {
        return $null
    }

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }

    return $property.Value
}

function Merge-JsonPatch {
    param(
        [AllowNull()][object] $Base,
        [AllowNull()][object] $Patch
    )

    if ($Patch -isnot [System.Management.Automation.PSCustomObject]) {
        return $Patch
    }

    $merged = [ordered]@{}
    if ($Base -is [System.Management.Automation.PSCustomObject]) {
        foreach ($property in $Base.PSObject.Properties) {
            $merged[$property.Name] = $property.Value
        }
    }

    foreach ($property in $Patch.PSObject.Properties) {
        if ($null -eq $property.Value) {
            $merged.Remove($property.Name)
            continue
        }

        $baseValue = if ($merged.Contains($property.Name)) { $merged[$property.Name] } else { $null }
        $merged[$property.Name] = Merge-JsonPatch -Base $baseValue -Patch $property.Value
    }

    return [pscustomobject]$merged
}

function Assert-ExactStrings {
    param(
        [AllowNull()][object[]] $Actual,
        [Parameter(Mandatory)][string[]] $Expected,
        [Parameter(Mandatory)][string] $Subject
    )

    $actualStrings = @($Actual | ForEach-Object { [string] $_ } | Sort-Object -Unique)
    $expectedStrings = @($Expected | Sort-Object -Unique)
    $difference = @(Compare-Object -ReferenceObject $expectedStrings -DifferenceObject $actualStrings)
    if ($difference.Count -ne 0 -or $actualStrings.Count -ne $Expected.Count) {
        Fail "$Subject must be exactly [$($Expected -join ', ')], got [$($actualStrings -join ', ')]"
    }
}

function Get-OnlyBuildArtifacts {
    param([Parameter(Mandatory)][string] $Root)

    if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
        Fail "target directory does not exist: $Root"
    }

    $outDirectories = @(
        Get-ChildItem -LiteralPath $Root -Recurse -Directory |
            Where-Object { $_.FullName -match "[\\/]build[\\/]secrets-storage-[^\\/]+[\\/]out$" }
    )
    if ($outDirectories.Count -ne 1) {
        Fail "target must contain exactly one secrets-storage build out directory, found $($outDirectories.Count)"
    }

    $capabilityFile = Join-Path $outDirectories[0].FullName "capabilities.json"
    $aclFile = Join-Path $outDirectories[0].FullName "acl-manifests.json"
    if (-not (Test-Path -LiteralPath $capabilityFile -PathType Leaf)) {
        Fail "generated capabilities.json is missing from the only build out directory"
    }
    if (-not (Test-Path -LiteralPath $aclFile -PathType Leaf)) {
        Fail "generated acl-manifests.json is missing from the only build out directory"
    }

    return [pscustomobject]@{
        Capabilities = $capabilityFile
        Acl = $aclFile
    }
}

function Assert-ProofCapability {
    param(
        [Parameter(Mandatory)][object] $Capability,
        [Parameter(Mandatory)][string] $Subject
    )

    if ((Get-Property $Capability "identifier") -ne "security-proof") {
        Fail "$Subject identifier must be security-proof"
    }
    if ((Get-Property $Capability "local") -ne $true) {
        Fail "$Subject must be local-only"
    }
    Assert-ExactStrings -Actual @(Get-Property $Capability "windows") -Expected @("security-proof") -Subject "$Subject windows"
    Assert-ExactStrings -Actual @(Get-Property $Capability "permissions") -Expected @(
        "allow-proof-install-canary",
        "allow-proof-authorized-probe",
        "allow-proof-lock",
        "allow-proof-status"
    ) -Subject "$Subject permissions"
    if ($null -ne (Get-Property $Capability "remote")) {
        Fail "$Subject must not define remote URLs"
    }
}

try {
    $baseConfig = Read-Json -Path $BaseConfigPath
    $proofOverlay = Read-Json -Path $OverlayConfigPath
    $proofCapability = Read-Json -Path $ProofCapabilityPath
    Assert-ProofCapability -Capability $proofCapability -Subject "source proof capability"

    $effectiveConfig = if ($Mode -eq "SecurityProof") {
        Merge-JsonPatch -Base $baseConfig -Patch $proofOverlay
    }
    else {
        $baseConfig
    }

    $windows = @(Get-Property (Get-Property $effectiveConfig "app") "windows")
    $windowLabels = @($windows | ForEach-Object { Get-Property $_ "label" })
    $security = Get-Property (Get-Property $effectiveConfig "app") "security"
    $activeCapabilities = @(Get-Property $security "capabilities")
    $features = @(
        $CargoFeatures -split "[,\s]+" |
            ForEach-Object { $_.Trim().ToLowerInvariant() } |
            Where-Object { $_ }
    )
    $hasProofFeature = $features -contains "security-proof"

    $artifacts = Get-OnlyBuildArtifacts -Root $TargetDir
    $generatedCapabilities = Read-Json -Path $artifacts.Capabilities
    $aclText = Get-Content -LiteralPath $artifacts.Acl -Raw
    $generatedAcl = $aclText | ConvertFrom-Json

    if ($Mode -eq "Normal") {
        Assert-ExactStrings -Actual $windowLabels -Expected @("main") -Subject "normal window labels"
        Assert-ExactStrings -Actual $activeCapabilities -Expected @("default") -Subject "normal active capabilities"
        if ($hasProofFeature) {
            Fail "normal mode must not enable the security-proof Cargo feature"
        }
        if ($null -ne (Get-Property $generatedCapabilities "security-proof")) {
            Fail "normal generated capabilities contain security-proof"
        }
        if ($aclText -match "proof[_-]") {
            Fail "normal generated ACL contains proof commands or permissions"
        }
    }
    else {
        Assert-ExactStrings -Actual $windowLabels -Expected @("security-proof") -Subject "proof window labels"
        Assert-ExactStrings -Actual $activeCapabilities -Expected @("security-proof") -Subject "proof active capabilities"
        if (-not $hasProofFeature) {
            Fail "proof mode requires the security-proof Cargo feature"
        }

        $generatedProofCapability = Get-Property $generatedCapabilities "security-proof"
        if ($null -eq $generatedProofCapability) {
            Fail "generated capabilities do not contain security-proof"
        }
        Assert-ProofCapability -Capability $generatedProofCapability -Subject "generated proof capability"

        $appAcl = Get-Property $generatedAcl "__app-acl__"
        $appPermissions = Get-Property $appAcl "permissions"
        if ($null -eq $appPermissions) {
            Fail "generated ACL does not contain app permissions"
        }

        $expectedCommands = @(
            "proof_authorized_probe",
            "proof_install_canary",
            "proof_lock",
            "proof_status"
        )
        $expectedPermissionIds = @()
        foreach ($command in $expectedCommands) {
            $slug = $command.Replace("_", "-")
            $expectedPermissionIds += "allow-$slug", "deny-$slug"
        }
        Assert-ExactStrings -Actual @($appPermissions.PSObject.Properties.Name) -Expected $expectedPermissionIds -Subject "generated app ACL permission ids"

        foreach ($command in $expectedCommands) {
            $slug = $command.Replace("_", "-")
            $allow = Get-Property $appPermissions "allow-$slug"
            $deny = Get-Property $appPermissions "deny-$slug"
            Assert-ExactStrings -Actual @(Get-Property (Get-Property $allow "commands") "allow") -Expected @($command) -Subject "allow-$slug commands"
            Assert-ExactStrings -Actual @(Get-Property (Get-Property $deny "commands") "deny") -Expected @($command) -Subject "deny-$slug commands"
        }
    }

    Write-Host "Effective Tauri config verified for mode $Mode."
    exit 0
}
catch {
    Write-Error $_.Exception.Message
    exit 1
}
