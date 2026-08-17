#!/bin/bash

# Script de développement pour Ratio Spoof

set -e

echo "🚀 Starting Ratio Spoof in development mode..."

# Check if Tauri CLI is installed
if ! cargo tauri --version &> /dev/null; then
    echo "📦 Installing Tauri CLI (this may take a few minutes)..."
    cargo install tauri-cli --version "^2.0.0"
fi

# Check if frontend dependencies are installed
if [ ! -d "frontend/node_modules" ]; then
    echo "📦 Installing frontend dependencies..."
    cd frontend
    npm install
    cd ..
fi

echo "🏃 Launching Tauri dev server..."
cd crates/app
cargo tauri dev
