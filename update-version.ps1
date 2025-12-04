#Requires -Version 3.0

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$version,
    
    [Parameter(Mandatory=$true)]
    [ValidateRange(0, 999)]
    [int]$major,
    
    [Parameter(Mandatory=$true)]
    [ValidateRange(0, 999)]
    [int]$minor,
    
    [Parameter(Mandatory=$true)]
    [ValidateRange(0, 999)]
    [int]$patch
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$versionWithBuild = "$version.0"

# Update RAC.rc
$rcPath = Join-Path $PSScriptRoot "icon\RAC.rc"

if (-not (Test-Path $rcPath)) {
    Write-Error "RAC.rc not found at: $rcPath"
    exit 1
}

try {
    Write-Host "[INFO] Updating RAC.rc..." -ForegroundColor Cyan
    
    $rcContent = Get-Content $rcPath -Raw -Encoding UTF8
    
    $rcContent = $rcContent -replace '(?m)^#define VER_MAJOR\s+\d+', "#define VER_MAJOR $major"
    $rcContent = $rcContent -replace '(?m)^#define VER_MINOR\s+\d+', "#define VER_MINOR $minor"
    $rcContent = $rcContent -replace '(?m)^#define VER_PATCH\s+\d+', "#define VER_PATCH $patch"
    $rcContent = $rcContent -replace '(?m)^#define VER_FILEVERSION_STR\s+".+"', "#define VER_FILEVERSION_STR `"$versionWithBuild`""
    $rcContent = $rcContent -replace '(?m)^#define VER_PRODUCTVERSION_STR\s+".+"', "#define VER_PRODUCTVERSION_STR `"$version`""

    $rcContent | Set-Content $rcPath -Encoding UTF8 -NoNewline

    Write-Host "[OK] RAC.rc updated" -ForegroundColor Green
}
catch {
    Write-Error "Failed to update RAC.rc: $_"
    exit 1
}

# Update manifest.xml
$manifestPath = Join-Path $PSScriptRoot "icon\manifest.xml"

if (-not (Test-Path $manifestPath)) {
    Write-Error "manifest.xml not found at: $manifestPath"
    exit 1
}

try {
    Write-Host "[INFO] Updating manifest.xml..." -ForegroundColor Cyan

    $manifestContent = Get-Content $manifestPath -Raw -Encoding UTF8
    $manifestContent = $manifestContent -replace 'version="\d+\.\d+\.\d+\.\d+"', "version=`"$versionWithBuild`""
    $manifestContent | Set-Content $manifestPath -Encoding UTF8 -NoNewline

    Write-Host "[OK] manifest.xml updated" -ForegroundColor Green
}
catch {
    Write-Error "Failed to update manifest.xml: $_"
    exit 1
}

Write-Host "[SUCCESS] Version updated to $version" -ForegroundColor Green
exit 0