<#[
.SYNOPSIS
Cria uma branch de release e sincroniza a versão do aplicativo.

.EXAMPLE
.\scripts\new-release.ps1 -Version 0.1.0-alpha.1
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory, Position = 0)]
    [string]$Version
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-Git {
    param(
        [Parameter(Mandatory)]
        [string[]]$Arguments,
        [int[]]$AllowedExitCodes = @(0)
    )

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $allOutput = @(& git @Arguments 2>&1)
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $stdout = @($allOutput | Where-Object { $_ -isnot [Management.Automation.ErrorRecord] } | ForEach-Object { $_.ToString() })
    if ($exitCode -notin $AllowedExitCodes) {
        $details = @($allOutput | ForEach-Object { $_.ToString() }) -join [Environment]::NewLine
        throw "git $($Arguments -join ' ') falhou:`n$details"
    }

    return $stdout
}

function Replace-Version {
    param(
        [Parameter(Mandatory)]
        [string]$Content,
        [Parameter(Mandatory)]
        [regex]$Pattern,
        [Parameter(Mandatory)]
        [string]$ExpectedCurrentVersion,
        [Parameter(Mandatory)]
        [string]$Path
    )

    $match = $Pattern.Match($Content)
    if (-not $match.Success) {
        throw "Não foi possível localizar a versão em $Path."
    }
    if ($match.Groups[2].Value -ne $ExpectedCurrentVersion) {
        throw "Versão divergente em ${Path}: $($match.Groups[2].Value), esperada $ExpectedCurrentVersion."
    }

    return $Pattern.Replace($Content, "`${1}$Version`${3}", 1)
}

$semVerPattern = '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*)?$'
if ($Version -notmatch $semVerPattern) {
    throw "Versão inválida: '$Version'. Use SemVer sem prefixo v e sem metadados +build."
}

$repoRoot = (Invoke-Git -Arguments @('rev-parse', '--show-toplevel') | Select-Object -First 1).Trim()
$branchName = "chore/release-v$Version"
$tagName = "v$Version"
$utf8WithoutBom = [Text.UTF8Encoding]::new($false)

Push-Location $repoRoot
try {
    if (@(Invoke-Git -Arguments @('status', '--porcelain')).Count -gt 0) {
        throw 'A árvore de trabalho precisa estar limpa antes de criar a release branch.'
    }

    $currentBranch = (Invoke-Git -Arguments @('branch', '--show-current') | Select-Object -First 1).Trim()
    if ($currentBranch -ne 'main') {
        throw "Execute o script na branch main; branch atual: '$currentBranch'."
    }

    Invoke-Git -Arguments @('fetch', '--prune', 'origin', 'main') | Out-Null
    $localMain = (Invoke-Git -Arguments @('rev-parse', 'HEAD') | Select-Object -First 1).Trim()
    $remoteMain = (Invoke-Git -Arguments @('rev-parse', 'refs/remotes/origin/main') | Select-Object -First 1).Trim()
    if ($localMain -ne $remoteMain) {
        throw 'A main local diverge de origin/main. Sincronize-a antes de criar a release branch.'
    }

    if (@(Invoke-Git -Arguments @('branch', '--list', $branchName)).Count -gt 0) {
        throw "A branch local '$branchName' já existe."
    }
    if (@(Invoke-Git -Arguments @('tag', '--list', $tagName)).Count -gt 0) {
        throw "A tag local '$tagName' já existe."
    }

    $remoteBranch = Invoke-Git -Arguments @('ls-remote', '--exit-code', '--heads', 'origin', $branchName) -AllowedExitCodes @(0, 2)
    if (@($remoteBranch).Count -gt 0) {
        throw "A branch remota '$branchName' já existe."
    }
    $remoteTag = Invoke-Git -Arguments @('ls-remote', '--exit-code', '--tags', 'origin', "refs/tags/$tagName") -AllowedExitCodes @(0, 2)
    if (@($remoteTag).Count -gt 0) {
        throw "A tag remota '$tagName' já existe."
    }

    $paths = @{
        Package = Join-Path $repoRoot 'package.json'
        Tauri = Join-Path $repoRoot 'src-tauri/tauri.conf.json'
        Cargo = Join-Path $repoRoot 'src-tauri/Cargo.toml'
        CargoLock = Join-Path $repoRoot 'src-tauri/Cargo.lock'
    }
    $original = @{}
    foreach ($key in $paths.Keys) {
        $original[$key] = [IO.File]::ReadAllText($paths[$key])
    }

    $jsonPattern = [regex]::new('(?m)^(\s*"version"\s*:\s*")([^"]+)(")')
    $cargoPattern = [regex]::new('(?ms)(^\[package\]\r?\n(?:(?!^\[).)*?^version\s*=\s*")([^"]+)(")')
    $cargoLockPattern = [regex]::new('(?ms)(^\[\[package\]\]\r?\nname\s*=\s*"secrets-storage"\r?\nversion\s*=\s*")([^"]+)(")')
    $currentVersion = $jsonPattern.Match($original.Tauri).Groups[2].Value
    if ([string]::IsNullOrWhiteSpace($currentVersion)) {
        throw 'Não foi possível ler a versão canônica de src-tauri/tauri.conf.json.'
    }

    $updated = @{
        Package = Replace-Version $original.Package $jsonPattern $currentVersion $paths.Package
        Tauri = Replace-Version $original.Tauri $jsonPattern $currentVersion $paths.Tauri
        Cargo = Replace-Version $original.Cargo $cargoPattern $currentVersion $paths.Cargo
        CargoLock = Replace-Version $original.CargoLock $cargoLockPattern $currentVersion $paths.CargoLock
    }

    Invoke-Git -Arguments @('switch', '-c', $branchName) | Out-Null
    try {
        foreach ($key in $paths.Keys) {
            [IO.File]::WriteAllText($paths[$key], $updated[$key], $utf8WithoutBom)
        }
    }
    catch {
        foreach ($key in $paths.Keys) {
            [IO.File]::WriteAllText($paths[$key], $original[$key], $utf8WithoutBom)
        }
        Invoke-Git -Arguments @('switch', 'main') | Out-Null
        Invoke-Git -Arguments @('branch', '-D', $branchName) | Out-Null
        throw
    }

    Write-Host "Release branch criada: $branchName" -ForegroundColor Green
    Write-Host "Versão sincronizada: $currentVersion -> $Version"
    Write-Host 'Próximos passos: revise o CHANGELOG.md, execute pnpm check e crie o commit:'
    Write-Host "  git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json CHANGELOG.md"
    Write-Host "  git commit -m `"chore(release): prepara $tagName`""
}
finally {
    Pop-Location
}
