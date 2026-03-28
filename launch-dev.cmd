@echo off
set SCRIPT_DIR=%~dp0
pwsh -ExecutionPolicy Bypass -File "%SCRIPT_DIR%launch-dev.ps1"
pause
