#!/bin/bash

# Script de build pour Ratio Spoof (macOS/Linux)

set -e

echo "🚀 Building Pierro Spoof..."

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install it first."
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install it first."
    exit 1
fi

# Check if Tauri CLI is installed
if ! cargo tauri --version &> /dev/null; then
    echo "📦 Installing Tauri CLI..."
    cargo install tauri-cli --version "^2.0.0"
fi

echo "📦 Installing frontend dependencies..."
cd frontend
npm install

echo "🏗️  Building application..."
cd ../crates/app
cargo tauri build

echo "✅ Build complete!"
echo ""
echo "📍 Binaries location:"
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "   macOS app: target/release/bundle/macos/Pierro Spoof.app"
    echo "   DMG installer: target/release/bundle/dmg/"
else
    echo "   Binary: target/release/ratio-spoof-app.exe"
fi
