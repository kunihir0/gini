# Development Environment Setup

## Overview

This guide will help you set up your development environment for working with the Gini project. It covers prerequisites, installation, and initial configuration.

## Prerequisites

### System Requirements

- **Operating System**: Linux (Ubuntu 20.04 or newer recommended)
- **CPU**: x86_64 with virtualization support (AMD-V or Intel VT-x)
- **RAM**: 8GB minimum, 16GB or more recommended
- **Storage**: 20GB free space minimum

### Required Software

- **Rust**: 1.70.0 or newer
- **QEMU/KVM**: 6.0 or newer
- **Cargo**: Latest version
- **Git**: For version control
- **Tokio**: Async runtime for Rust
- **LVM2**: For VM storage management

## Installing Prerequisites

### Rust and Cargo

Install Rust and Cargo using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify the installation:

```bash
rustc --version
cargo --version
```

### QEMU and KVM

Install QEMU and KVM packages:

```bash
sudo apt update
sudo apt install qemu-kvm libvirt-daemon-system libvirt-clients bridge-utils
```

Verify that KVM is available:

```bash
kvm-ok
```

### LVM2

Install LVM2:

```bash
sudo apt install lvm2
```

## Setting Up the Project

### Clone the Repository

Clone the Gini repository:

```bash
git clone https://github.com/kunihir0/gini.git
cd gini
```

### Build the Project

Build the project using Cargo:

```bash
cargo build
```

For optimized release builds:

```bash
cargo build --release
```

## Configuration

### Basic Configuration

The system uses the `./user/` directory for user-specific data and configurations:

- `./user/config/` - Configuration files
- `./user/data/` - Data files
- `./user/plugins/` - User-installed plugins

### Environment Variables

You can customize certain aspects using environment variables:

- `GINI_BASE_PATH` - Override the base path (default: current directory)
- `GINI_LOG_LEVEL` - Set logging level (default: info)
- `GINI_PLUGIN_PATH` - Set additional plugin search paths

## Development Tools

### IDE Integration

For Visual Studio Code:
1. Install the "rust-analyzer" extension
2. Install the "crates" extension
3. Configure `.vscode/settings.json`:

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.procMacro.enable": true
}
```

### Linting and Formatting

Install clippy and rustfmt:

```bash
rustup component add clippy rustfmt
```

Run linting:

```bash
cargo clippy
```

Format code:

```bash
cargo fmt
```

## Running the Project

Run the project directly:

```bash
cargo run
```

With additional arguments:

```bash
cargo run -- --config path/to/config.json
```

## Troubleshooting

### Common Issues

1. **Missing KVM support**: Ensure that virtualization is enabled in BIOS/UEFI
2. **Permission errors**: Ensure you're in the `kvm` and `libvirt` groups:
   ```bash
   sudo usermod -a -G kvm,libvirt $(whoami)
   ```
3. **Build failures**: Update Rust:
   ```bash
   rustup update
   ```

### Getting Help

If you encounter issues:
1. Check the existing issues in the GitHub repository
2. Ask for help in the developer chat
3. Create a new issue with detailed information about your problem