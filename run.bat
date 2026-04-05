@echo off
:: ╔════════════════════════════════════════╗
:: ║  EXSUL — Production launcher           ║
:: ║  Runs the pre-built release binary.    ║
:: ║  Frontend is embedded inside the exe. ║
:: ╚════════════════════════════════════════╝
title Exsul
cd /d "%~dp0"

set EXE=%~dp0src-tauri\target\release\exsul.exe

if exist "%EXE%" (
    start "" "%EXE%"
    exit /b 0
)

echo.
echo  [Exsul] No release binary found.
echo  [Exsul] Building now — this takes a few minutes on first run...
echo.

call npm run tauri build -- --no-bundle
if errorlevel 1 (
    echo.
    echo  [Exsul] Build failed. See output above.
    pause
    exit /b 1
)

start "" "%EXE%"
