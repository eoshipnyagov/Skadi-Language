# Installing Skadi CLI

Current release candidate: `v1.2.0-rc.1`.

The installer places a prebuilt `skadi-cli` in a user-local directory, verifies
the archive SHA-256, and records an installation manifest. Rust and Cargo are
not required for normal use after installation.

!!! note
    Skadi generates C, so `build` and `run` require an external C compiler.
    The installer deliberately does not modify the system toolchain. Run
    `skadi-cli doctor` for platform-specific guidance.

## Windows

Open PowerShell:

```powershell
$installer = Join-Path $env:TEMP "skadi-install.ps1"
Invoke-WebRequest `
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.ps1 `
  -OutFile $installer
& $installer -Version 1.2.0-rc.1
```

The default binary location is:

```text
%LOCALAPPDATA%\Programs\Skadi\bin\skadi-cli.exe
```

The installer updates the user `PATH`, not the system `PATH`. Restart the
terminal, then verify the environment:

```powershell
skadi-cli --version
skadi-cli doctor
```

Use either MSVC Build Tools (`cl`) or MSYS2/MinGW-w64 (`gcc`) for native builds.

## Linux

```bash
curl -fsSL \
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.sh \
  | sh -s -- --version 1.2.0-rc.1
```

The default installation is a statically linked musl binary at
`~/.local/bin/skadi-cli`. Both `x86_64` and `aarch64` are supported.

Use `--system` to install into `/usr/local/bin`. The installer uses `sudo` only
for an explicit system installation.

Install the host C toolchain through your distribution:

```bash
# Debian / Ubuntu
sudo apt install build-essential

# Fedora
sudo dnf group install "Development Tools"

# Arch Linux
sudo pacman -S base-devel

# Alpine
sudo apk add build-base
```

## macOS

Use the same POSIX installer:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.sh \
  | sh -s -- --version 1.2.0-rc.1
```

Separate archives are provided for Intel (`x86_64`) and Apple Silicon
(`aarch64`). Install the C toolchain with:

```bash
xcode-select --install
```

RC binaries are not yet signed with an Apple Developer ID or notarized. The
installer does not disable Gatekeeper or remove quarantine attributes. If your
organization blocks unsigned command-line tools, build `skadi-cli` from source
or wait for a signed release.

## First project

From a working directory:

```bash
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format --check
skadi-cli build
skadi-cli run
```

For the interactive workflow:

```bash
skadi-cli tui
```

## Versions and upgrades

Running the installer again with a new `--version` atomically replaces the
managed binary and updates the manifest. Projects and `Skadi.toml` files are
not touched.

Without `--version`, the installer resolves the latest published GitHub
Release. Always select prerelease and RC versions explicitly.

## Offline installation

Download the platform archive and `SHA256SUMS` from GitHub Releases, then pass
both files to the installer.

Windows:

```powershell
.\install.ps1 `
  -Version 1.2.0-rc.1 `
  -ArchivePath .\skadi-cli-v1.2.0-rc.1-x86_64-pc-windows-msvc.zip `
  -ChecksumPath .\SHA256SUMS
```

Linux/macOS:

```bash
sh install.sh \
  --version 1.2.0-rc.1 \
  --archive ./skadi-cli-v1.2.0-rc.1-x86_64-unknown-linux-musl.tar.gz \
  --checksum-file ./SHA256SUMS
```

The local archive name must match its version and the current platform.

## Uninstalling

Windows:

```powershell
$uninstaller = Join-Path $env:TEMP "skadi-uninstall.ps1"
Invoke-WebRequest `
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/uninstall.ps1 `
  -OutFile $uninstaller
& $uninstaller
```

Linux/macOS:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/uninstall.sh \
  | sh
```

Uninstalling uses the manifest and removes only the managed binary and the
installer-owned PATH block. Projects, source files, and user configuration are
preserved.

## Building from source

This path is intended for compiler contributors:

```bash
cargo build --release -p skadi-cli
cargo run -p skadi-cli -- --version
```

For normal use, prefer a release archive and the installed `skadi-cli` command.
