# Build script for Windows
# Run this in PowerShell on Windows

Write-Host "Building Pierro Spoof for Windows..." -ForegroundColor Cyan

# Check prerequisites
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust not found. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Host "Node.js not found. Install from https://nodejs.org" -ForegroundColor Red
    exit 1
}

# Install frontend dependencies
Write-Host "Installing frontend dependencies..." -ForegroundColor Yellow
Set-Location frontend
npm install
Set-Location ..

# Build the application
Write-Host "Building application..." -ForegroundColor Yellow
Set-Location crates/app
cargo tauri build

Write-Host "`nBuild complete!" -ForegroundColor Green
Write-Host "Binaries are in: target/release/bundle/" -ForegroundColor Cyan
