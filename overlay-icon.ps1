param(
    [Parameter(Mandatory = $true)]
    [string]$BaseIco,

    [Parameter(Mandatory = $true)]
    [string]$OverlaySvg,

    [Parameter(Mandatory = $true)]
    [string]$OutputIco,

    [ValidateRange(0.01, 1.0)]
    [double]$Scale = 0.36,

    [ValidateSet("TopLeft", "TopRight", "BottomLeft", "BottomRight", "Center")]
    [string]$Anchor = "BottomRight",

    [ValidateRange(0.0, 0.49)]
    [double]$Margin = 0.06,

    [switch]$Release
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

function Resolve-AbsolutePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathValue
    )

    return [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $PathValue).Path)
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo is not installed or is not on PATH. Install Rust from https://rustup.rs/ and try again."
}

$manifestPath = Join-Path $scriptDir "tools\icon-overlay\Cargo.toml"
$resolvedBaseIco = Resolve-AbsolutePath -PathValue $BaseIco
$resolvedOverlaySvg = Resolve-AbsolutePath -PathValue $OverlaySvg
$resolvedOutputIco = if ([System.IO.Path]::IsPathRooted($OutputIco)) {
    [System.IO.Path]::GetFullPath($OutputIco)
}
else {
    [System.IO.Path]::GetFullPath((Join-Path $scriptDir $OutputIco))
}
$outputDir = Split-Path -Parent $resolvedOutputIco

if ($outputDir) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$anchorValue = switch ($Anchor) {
    "TopLeft" { "top-left" }
    "TopRight" { "top-right" }
    "BottomLeft" { "bottom-left" }
    "BottomRight" { "bottom-right" }
    "Center" { "center" }
}

$cargoArgs = @(
    "run"
    "--manifest-path", $manifestPath
)

if ($Release) {
    $cargoArgs += "--release"
}

$cargoArgs += "--"
$cargoArgs += @(
    "--base-ico", $resolvedBaseIco,
    "--overlay-svg", $resolvedOverlaySvg,
    "--output-ico", $resolvedOutputIco,
    "--scale", $Scale.ToString([System.Globalization.CultureInfo]::InvariantCulture),
    "--anchor", $anchorValue,
    "--margin", $Margin.ToString([System.Globalization.CultureInfo]::InvariantCulture)
)

& cargo @cargoArgs

if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Host "Overlay icon written to $resolvedOutputIco" -ForegroundColor Green
