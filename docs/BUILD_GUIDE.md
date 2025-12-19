# Build Guide

This guide explains how to build RAC Clicker from source on Windows using the provided portable build scripts.

## Prerequisites

- Windows (PowerShell available)
- Rust toolchain (recommended version: 1.92.0) install and ensure `cargo` is on PATH

Security note
- The wrapper uses `-ExecutionPolicy Bypass` only for the child PowerShell process (no permanent policy change).

## Files added for Windows builds

- build.ps1 - PowerShell build script (portable)
- build.bat - Wrapper that runs the PowerShell script with a process-scoped ExecutionPolicy Bypass

## How to build (recommended)

From the repository root, run the batch wrapper:

Open a Command Prompt and run:
```
.\build.bat
```

This runs the default (Release) build and writes artifacts to `.\dist`.

Example output files (default):
- dist\rac-clicker-v{version}.exe

## Options

You can pass the same parameters to the batch wrapper (they are forwarded to PowerShell):

- `-Configuration` - "Release" (default) or "Debug"
- `-OutDir` - specify a different output directory

Examples:

Release build (default):
```
build.bat -Configuration release
```

## Manual build (alternative)

If you prefer to run `cargo` directly:

```
cargo build --release
```

Then find the executable in:
```
target\release\rac-clicker.exe
```

### Build Fails
- Ensure you have the latest Rust toolchain
- Check internet connection for dependency downloads
- Try `cargo clean` then rebuild

### Missing Dependencies
All dependencies are automatically downloaded during build. No manual installation required.