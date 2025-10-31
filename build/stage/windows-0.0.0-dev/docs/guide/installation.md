# Installing Axion DSL

## Prerequisites

- Python 3.9+ (required for the lightweight runner and REPL)
- Rust toolchain (optional, only required when building the native CLI)

## Unix-like systems

```bash
./install.sh
export PATH="$HOME/.local/bin:$PATH"
axion plan examples/hello.ax --json
```

Set `PREFIX=/custom/prefix ./install.sh` to install elsewhere.

## Windows (PowerShell)

```powershell
./install.ps1
$env:PATH = "$env:USERPROFILE\AppData\Local\Axion\bin;" + $env:PATH
axion.cmd plan examples\hello.ax --json
```

Administrator privileges are not required; files are installed under the user profile.

## Verifying the installation

The command `axion --help` should display the Python runner options. The helper copies both the launcher and `axion_runner.py` into the same directory, allowing direct execution of `.ax` files that declare `#!/usr/bin/env axion`.

## Removal

Delete the installation directory (`$HOME/.local/bin` entries on Unix or `%USERPROFILE%\AppData\Local\Axion` on Windows) and remove the PATH modification from shell configuration files.

## Building local archives

When preparing a release or testing installers, use the helper scripts:

- `pwsh scripts/package/windows/package.ps1 -Version 0.1.0` — builds the CLI, stages resources, and produces `build/package/windows/axion-0.1.0-windows-x64.zip`.
- `VERSION=0.1.0 bash scripts/package/linux/package.sh` — produces `build/package/linux/axion-0.1.0-linux-x64.tar.gz`.

The generated archives contain the CLI binary, documentation, examples, and an `install` script for the respective platform.
