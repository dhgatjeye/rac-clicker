# Build Guide

This guide helps you build RAC Clicker from source on Windows.

## Prerequisites

### Install Rust
1. Download Rust(1.90.0 version) installer
2. Run the installer and follow the setup instructions
3. Restart your terminal or command prompt

## Building the Project

### 1. Clone the Repository
```
git clone https://github.com/dhgatjeye/rac-clicker.git
cd rac-clicker
```

### 2. Build Release Version
Run the build script:
```
build-release.bat
```

This will:
- Compile the project in release mode
- Create a versioned executable in `target/release/`

### 3. Manual Build (Alternative)
If you prefer manual building:
```
cargo build --release
```

The executable will be located at `target/release/rac-clicker.exe`

## Output

After successful build, you will find:
- `rac-clicker-vX.X.X.exe` - Versioned executable
- `rac-clicker.exe` - Standard executable

Both files are in the `target/release/` directory.

## Troubleshooting

### Build Fails
- Ensure you have the latest Rust toolchain
- Check internet connection for dependency downloads
- Try `cargo clean` then rebuild

### Missing Dependencies
All dependencies are automatically downloaded during build. No manual installation required.
