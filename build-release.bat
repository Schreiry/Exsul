@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul
title Exsul - Release Build

REM ============================================================
REM  EXSUL :: Release Build Script
REM  ----------------------------------------------------------
REM  Run this from the project root (the folder that contains
REM  package.json and src-tauri/).  It will:
REM    1. Verify pnpm/npm and Rust are installed
REM    2. Install JS dependencies if missing
REM    3. Build the SvelteKit frontend (release)
REM    4. Build the Tauri Rust backend (release)
REM    5. Bundle a Windows installer + plain .exe
REM    6. Copy artefacts into ./release/
REM
REM  No development tools are required on the target machine.
REM  Just install the produced .msi/.exe and run.
REM ============================================================

cd /d "%~dp0"

echo.
echo ============================================================
echo  EXSUL :: Release Build
echo  Working directory: %CD%
echo ============================================================
echo.

REM -- 1. Toolchain checks -------------------------------------------------
where cargo >nul 2>&1
if errorlevel 1 (
    echo [X] Rust toolchain not found.
    echo     Install from https://www.rust-lang.org/tools/install
    echo     then re-open this terminal.
    goto :fail
)

REM Prefer pnpm; fall back to npm so the script still works without pnpm.
set "PKG_MGR="
where pnpm >nul 2>&1 && set "PKG_MGR=pnpm"
if "%PKG_MGR%"=="" (
    where npm >nul 2>&1 && set "PKG_MGR=npm"
)
if "%PKG_MGR%"=="" (
    echo [X] Neither pnpm nor npm is installed.
    echo     Install Node.js LTS from https://nodejs.org/ first.
    goto :fail
)
echo [OK] Using package manager: %PKG_MGR%

REM -- 2. Install JS dependencies -----------------------------------------
if not exist "node_modules" (
    echo.
    echo [..] Installing JS dependencies (first run, this takes a few minutes)
    if /I "%PKG_MGR%"=="pnpm" (
        call pnpm install
    ) else (
        call npm install
    )
    if errorlevel 1 (
        echo [X] Dependency install failed.
        goto :fail
    )
) else (
    echo [OK] node_modules already present, skipping install
)

REM -- 3-5. Build the bundle ---------------------------------------------
echo.
echo [..] Building release bundle (SvelteKit + Rust + installer)
echo     This step can take 5-15 minutes the first time because Rust
echo     compiles every dependency from scratch.  Subsequent builds
echo     are much faster thanks to the target/ cache.
echo.

if /I "%PKG_MGR%"=="pnpm" (
    call pnpm tauri build
) else (
    call npm run tauri -- build
)
if errorlevel 1 (
    echo.
    echo [X] Tauri build failed - see messages above.
    goto :fail
)

REM -- 6. Collect artefacts ----------------------------------------------
set "BUNDLE_DIR=src-tauri\target\release\bundle"
set "BIN_PATH=src-tauri\target\release\Exsul.exe"
set "OUT_DIR=release"

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

echo.
echo [..] Collecting artefacts into .\%OUT_DIR%\

if exist "%BIN_PATH%" (
    copy /Y "%BIN_PATH%" "%OUT_DIR%\Exsul.exe" >nul
    echo [OK] Standalone .exe       -^> %OUT_DIR%\Exsul.exe
) else (
    echo [!] Standalone .exe not found at %BIN_PATH%
)

REM MSI installer (preferred for clients)
for /f "delims=" %%F in ('dir /b /s "%BUNDLE_DIR%\msi\*.msi" 2^>nul') do (
    copy /Y "%%F" "%OUT_DIR%\" >nul
    echo [OK] MSI installer        -^> %OUT_DIR%\%%~nxF
)

REM NSIS installer (lighter, single-exe setup)
for /f "delims=" %%F in ('dir /b /s "%BUNDLE_DIR%\nsis\*.exe" 2^>nul') do (
    copy /Y "%%F" "%OUT_DIR%\" >nul
    echo [OK] NSIS installer       -^> %OUT_DIR%\%%~nxF
)

echo.
echo ============================================================
echo  Build complete.
echo  Send the contents of .\%OUT_DIR%\ to your client.
echo  - Exsul.exe is a portable build (no install needed).
echo  - The .msi / setup.exe perform a regular install.
echo ============================================================
echo.

REM Open the release folder in Explorer so the user sees the result.
start "" "%CD%\%OUT_DIR%"

pause
exit /b 0

:fail
echo.
echo ============================================================
echo  Build FAILED.  Fix the error above and run this script
echo  again.  Nothing was copied to .\%OUT_DIR%\.
echo ============================================================
echo.
pause
exit /b 1
