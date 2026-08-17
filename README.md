# Pierro Spoof

A cross-platform BitTorrent ratio enhancement tool built with Rust and Tauri.

## Features

- Cross-platform (macOS, Windows)
- Modern dark UI with neon accents
- BitTorrent client emulation (qBittorrent profiles)
- Custom DNS resolution (bypasses local DNS blocking)
- Automatic tracker announce handling

## Building

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (LTS version)
- Platform-specific requirements:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools with C++

### Build Commands

#### macOS

```bash
# Install dependencies
cd frontend && npm install && cd ..

# Build the application
./build.sh
```

The app will be in `target/release/bundle/macos/Pierro Spoof.app`

#### Windows

```powershell
# Install dependencies
cd frontend
npm install
cd ..

# Build the application
cd crates/app
cargo tauri build
```

The installer will be in `target/release/bundle/msi/` or `target/release/bundle/nsis/`

## Development

```bash
# Start development server
cd crates/app
cargo tauri dev
```

## Project Structure

```
├── crates/
│   ├── app/          # Tauri application
│   └── core/         # Core library (tracker, spoofer, etc.)
├── frontend/         # React + TypeScript UI
└── build.sh          # macOS build script
```

## License

Private - All rights reserved
