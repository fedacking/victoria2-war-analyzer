param(
    [string]$OutDir = "artifacts\windows",
    [switch]$SkipInstall
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

function Require-Command {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$InstallHint
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "$Name is not installed or is not on PATH." -ForegroundColor Red
        Write-Host $InstallHint -ForegroundColor Yellow
        Read-Host "Press Enter to close"
        exit 1
    }
}

function Get-SafeArtifactName {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value
    )

    return (($Value -replace "[^A-Za-z0-9]+", "-").Trim("-")).ToLowerInvariant()
}

Require-Command -Name "cargo" -InstallHint "Install Rust from https://rustup.rs/ and reopen this script."
Require-Command -Name "node" -InstallHint "Install Node.js from https://nodejs.org/ and reopen this script."
Require-Command -Name "npm" -InstallHint "Install Node.js from https://nodejs.org/ and reopen this script."

$tauriConfig = Get-Content ".\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$productName = $tauriConfig.productName
$version = $tauriConfig.version
$artifactStem = Get-SafeArtifactName -Value $productName
$artifactDir = if ([System.IO.Path]::IsPathRooted($OutDir)) {
    [System.IO.Path]::GetFullPath($OutDir)
}
else {
    [System.IO.Path]::GetFullPath((Join-Path $scriptDir $OutDir))
}
$bundleDir = Join-Path $scriptDir "src-tauri\target\release\bundle\nsis"
$installerName = "$artifactStem-$version-windows-x64-setup.exe"
$installerArtifactPath = Join-Path $artifactDir $installerName
$checksumPath = "$installerArtifactPath.sha256.txt"

if (-not $SkipInstall -and -not (Test-Path ".\node_modules\@tauri-apps\cli")) {
    Write-Host "Installing frontend dependencies..." -ForegroundColor Cyan
    npm ci
    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "npm ci failed." -ForegroundColor Red
        Read-Host "Press Enter to close"
        exit $LASTEXITCODE
    }
}

Write-Host "Building installer for $productName $version..." -ForegroundColor Cyan
$buildStartedAt = Get-Date
npm run build:installer

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "Release build failed." -ForegroundColor Red
    Read-Host "Press Enter to close"
    exit $LASTEXITCODE
}

$installerCandidates = @()

if (Test-Path $bundleDir) {
    $installerCandidates = Get-ChildItem -Path $bundleDir -Filter *.exe -File -Recurse
}

$installer = $installerCandidates |
    Where-Object { $_.LastWriteTime -ge $buildStartedAt.AddSeconds(-5) } |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

if (-not $installer) {
    $installer = $installerCandidates |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
}

if (-not $installer) {
    Write-Host ""
    Write-Host "Build completed, but no NSIS installer executable was found in $bundleDir." -ForegroundColor Red
    Read-Host "Press Enter to close"
    exit 1
}

New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
Copy-Item -LiteralPath $installer.FullName -Destination $installerArtifactPath -Force

$hash = Get-FileHash -LiteralPath $installerArtifactPath -Algorithm SHA256
"$($hash.Hash.ToLowerInvariant()) *$installerName" | Set-Content -LiteralPath $checksumPath

Write-Host ""
Write-Host "Release build complete." -ForegroundColor Green
Write-Host "Installer: $installerArtifactPath" -ForegroundColor Green
Write-Host "Checksum:  $checksumPath" -ForegroundColor Green
