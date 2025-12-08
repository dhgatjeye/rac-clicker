@echo off
setlocal enabledelayedexpansion

:: ============================================
::          RAC CLICKER BUILD SCRIPT
:: ============================================

echo.
echo [BUILD] Starting build process...
echo.

:: Check if Cargo.toml exists
if not exist "Cargo.toml" (
    echo [ERROR] Cargo.toml not found in current directory
    exit /b 1
)

:: Extract version from Cargo.toml (only from [package] section)
set "inPackage="
for /f "tokens=*" %%a in (Cargo.toml) do (
    set "line=%%a"
    echo !line! | findstr /r "^\[package\]" >nul && set "inPackage=1"
    echo !line! | findstr /r "^\[" >nul && if not "!line!"=="[package]" set "inPackage="

    if defined inPackage (
        echo !line! | findstr /r "^version.*=" >nul && (
            for /f "tokens=2 delims==^" %%b in ("%%a") do set "version=%%b"
            goto :version_found
        )
    )
)
:version_found

:: Clean version string
set "version=%version: =%"
set "version=%version:"=%"

if "%version%"=="" (
    echo [ERROR] Could not extract version from Cargo.toml
    exit /b 1
)

echo [INFO] Version: %version%

:: Parse version components
for /f "tokens=1-3 delims=." %%a in ("%version%") do (
    set "major=%%a"
    set "minor=%%b"
    set "patch=%%c"
)

:: Validate version components
if "%major%"=="" set "major=0"
if "%minor%"=="" set "minor=0"
if "%patch%"=="" set "patch=0"

:: Update version in resource files
echo [INFO] Updating resource files...
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "update-version.ps1" -version "%version%" -major "%major%" -minor "%minor%" -patch "%patch%"

if errorlevel 1 (
    echo [ERROR] Failed to update version files
    exit /b 1
)

:: Build release
echo.
echo [BUILD] Compiling release build...
cargo build --release

if errorlevel 1 (
    echo [ERROR] Build failed
    exit /b 1
)

:: Copy and rename executable
set "sourcePath=target\release\rac-clicker.exe"
set "targetPath=target\release\rac-clicker-v%version%.exe"

if not exist "%sourcePath%" (
    echo [ERROR] Build output not found: %sourcePath%
    exit /b 1
)

echo [INFO] Creating versioned executable...

if exist "%targetPath%" del /f /q "%targetPath%" 2>nul

copy /y "%sourcePath%" "%targetPath%" >nul

if not exist "%targetPath%" (
    echo [ERROR] Failed to create versioned executable
    exit /b 1
)

:: Success
echo.
echo ============================================
echo [SUCCESS] Build completed successfully!
echo ============================================
echo.
echo Version:  %version%
echo Output:   %targetPath%
echo Size:
for %%F in ("%targetPath%") do echo           %%~zF bytes
echo.

exit /b 0