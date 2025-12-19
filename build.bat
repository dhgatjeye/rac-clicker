@echo off
SET SCRIPT_DIR=%~dp0
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%build.ps1" %*
IF ERRORLEVEL 1 (
  echo Build failed with exit code %ERRORLEVEL%
  exit /b %ERRORLEVEL%
)
exit /b 0