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

Require-Command -Name "cargo" -InstallHint "Install Rust from https://rustup.rs/ and reopen this script."
Require-Command -Name "node" -InstallHint "Install Node.js from https://nodejs.org/ and reopen this script."
Require-Command -Name "npm" -InstallHint "Install Node.js from https://nodejs.org/ and reopen this script."

if (-not (Test-Path ".\node_modules")) {
    Write-Host "Installing frontend dependencies..." -ForegroundColor Cyan
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "npm install failed." -ForegroundColor Red
        Read-Host "Press Enter to close"
        exit $LASTEXITCODE
    }
}

Write-Host "Launching Victoria 2 War Analyzer..." -ForegroundColor Green
npm run tauri dev

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "Launch failed." -ForegroundColor Red
    Read-Host "Press Enter to close"
    exit $LASTEXITCODE
}
