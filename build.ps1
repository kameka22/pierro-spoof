# Script de build pour Ratio Spoof (Windows)

Write-Host "🚀 Building Ratio Spoof..." -ForegroundColor Green

# Check if Node.js is installed
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Node.js is not installed. Please install it first." -ForegroundColor Red
    exit 1
}

# Check if Rust is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust is not installed. Please install it first." -ForegroundColor Red
    exit 1
}

Write-Host "📦 Installing frontend dependencies..." -ForegroundColor Cyan
Set-Location frontend
npm install

Write-Host "🏗️  Building application..." -ForegroundColor Cyan
Set-Location ..\crates\app
cargo tauri build

Write-Host "✅ Build complete!" -ForegroundColor Green
Write-Host ""
Write-Host "📍 Binaries location:" -ForegroundColor Yellow
Write-Host "   Executable: crates\app\target\release\ratio-spoof-app.exe"
Write-Host "   MSI installer: crates\app\target\release\bundle\msi\"
