param(
    [string]$Configuration = "release",
    [string]$OutDir = ".\dist"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Info($m) { Write-Host ("[INFO] {0}" -f $m) -ForegroundColor White }
function Write-Warn($m) { Write-Host ("[WARN] {0}" -f $m) -ForegroundColor Yellow }
function Write-Err($m)  { Write-Host ("[ERROR] {0}" -f $m) -ForegroundColor Red }

# Determine repository root (this script's directory or current dir)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if ([string]::IsNullOrEmpty($ScriptDir)) {
    $RepoRoot = Get-Location
} else {
    $RepoRoot = Resolve-Path $ScriptDir
}
Set-Location $RepoRoot

Write-Info ("Repository root: {0}" -f $RepoRoot.Path)

# Ensure cargo exists
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Err "cargo not found in PATH. Install Rust toolchain and ensure 'cargo' is on PATH."
    exit 2
}

# Run cargo build
Write-Info ("Running cargo build --{0} ..." -f $Configuration)
$buildStart = Get-Date
$proc = Start-Process -FilePath "cargo" -ArgumentList "build","--$Configuration" -NoNewWindow -Wait -PassThru
if ($proc.ExitCode -ne 0) {
    Write-Err ("cargo build failed with exit code {0}" -f $proc.ExitCode)
    exit $proc.ExitCode
}
$buildEnd = Get-Date
Write-Info ("Build finished in {0:N1}s" -f (($buildEnd - $buildStart).TotalSeconds))

# Parse Cargo.toml for package.name and package.version
$cargoToml = Join-Path $RepoRoot "Cargo.toml"
if (-not (Test-Path $cargoToml)) {
    Write-Err ("Cargo.toml not found at {0}" -f $cargoToml)
    exit 3
}

$name = $null; $version = $null; $inPackage = $false
Get-Content $cargoToml | ForEach-Object {
    $line = $_.Trim()
    if ($line -match '^\[package\]') { $inPackage = $true; return }
    if ($inPackage -and $line -match '^\[') { $inPackage = $false; return }
    if ($inPackage) {
        if ($null -eq $name -and $line -match '^name\s*=\s*"(.*)"') { $name = $matches[1] }
        if ($null -eq $version -and $line -match '^version\s*=\s*"(.*)"') { $version = $matches[1] }
    }
}

if (-not $name -or -not $version) {
    Write-Err "Failed to parse package name/version from Cargo.toml"
    exit 4
}

Write-Info ("Package: {0}, Version: {1}" -f $name, $version)

# Determine built artifact path (Windows)
$exeName = "$name.exe"
$targetDir = Join-Path $RepoRoot "target\$Configuration"
$builtExe = Join-Path $targetDir $exeName

if (-not (Test-Path $builtExe)) {
    # sometimes binary name may differ (workspace) search for it
    $found = Get-ChildItem -Path $targetDir -Filter "*.exe" -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.BaseName -eq $name }
    if ($found) { $builtExe = $found[0].FullName }
}

if (-not (Test-Path $builtExe)) {
    Write-Err ("Built executable not found at expected location: {0}" -f $builtExe)
    Write-Host ("Contents of {0}:" -f $targetDir)
    Get-ChildItem $targetDir -Recurse | ForEach-Object { Write-Host $_.FullName }
    exit 5
}

# Prepare output dir
$OutDirFull = Resolve-Path -Path $OutDir -ErrorAction SilentlyContinue
if (-not $OutDirFull) { New-Item -ItemType Directory -Path $OutDir -Force | Out-Null; $OutDirFull = Resolve-Path $OutDir }
$OutDirFull = $OutDirFull.Path

# Build output filename and copy
$artifactName = ("{0}-v{1}.exe" -f $name, $version)
$artifactPath = Join-Path $OutDirFull $artifactName
Copy-Item -Path $builtExe -Destination $artifactPath -Force
Write-Info ("Copied built artifact to: {0}" -f $artifactPath)

Write-Info "Build script completed successfully."
exit 0