@echo off
setlocal EnableExtensions

rem ============================================================
rem  Dorm Game Hub - launcher start script (Windows).
rem  Checks the toolchain, installs npm deps if needed, then runs
rem  the Tauri app.
rem
rem  Usage (double-click friendly):
rem    start.bat          install deps if missing, then `tauri dev`
rem    start.bat dev      same as above
rem    start.bat build    build a release bundle (`tauri build`)
rem ============================================================

set "ROOT=%~dp0"
pushd "%ROOT%"

if not exist "src-tauri\tauri.conf.json" (
    echo [ERROR] Run this script from the repository root ^(src-tauri not found^).
    goto :fail
)

where node >nul 2>nul
if errorlevel 1 goto :no_node

where cargo >nul 2>nul
if errorlevel 1 goto :no_cargo

if not exist "node_modules" (
    echo [setup] installing npm dependencies...
    call npm install
    if errorlevel 1 goto :npm_fail
)

if /I "%~1"=="build" goto :build

echo [run] tauri dev
call npm run tauri dev
goto :end

:build
echo [run] tauri build
call npm run tauri build
goto :end

:no_node
echo [ERROR] Node.js not found. Install Node 20+ from https://nodejs.org/
goto :fail

:no_cargo
echo [ERROR] Rust/Cargo not found. Install from https://rustup.rs/
goto :fail

:npm_fail
echo [ERROR] npm install failed.
goto :fail

:fail
popd
pause
exit /b 1

:end
popd
exit /b 0
