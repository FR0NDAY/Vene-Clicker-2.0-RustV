@echo off
echo Building VeneClicker (release, exe-only output)...
powershell -ExecutionPolicy Bypass -File scripts\\build_release.ps1
if %errorlevel% neq 0 (
    echo.
    echo Build failed! Please ensure Rust is installed (https://rustup.rs).
    pause
    exit /b %errorlevel%
)
echo.
echo Build successful!
echo Exe: target\\release\\vene_clicker.exe
pause
