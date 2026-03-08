@echo off
echo Building VeneClicker (Rust)...
cargo build --release
if %errorlevel% neq 0 (
    echo.
    echo Build failed! Please ensure Rust is installed (https://rustup.rs).
    pause
    exit /b %errorlevel%
)
echo.
echo Build successful!
echo Launching VeneClicker...
echo.
cargo run --release
pause
