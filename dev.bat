@echo off
:: ╔════════════════════════════════════════════════════════╗
:: ║  EXSUL — Development mode                              ║
:: ║  Starts the Vite dev server + Tauri app together.      ║
:: ║  Hot-reload is active. Changes reflect instantly.      ║
:: ╚════════════════════════════════════════════════════════╝
title Exsul — Dev
cd /d "%~dp0"

if not exist "node_modules" (
    echo  [Exsul] Installing dependencies...
    call npm install
)

npm run tauri dev
