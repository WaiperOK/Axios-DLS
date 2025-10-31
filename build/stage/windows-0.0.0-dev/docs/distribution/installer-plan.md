# Axion Distribution Plan

This document captures the implementation plan for roadmap section **7. Distribution & Tooling**. It breaks the work into deliverables, platform specifics, and supporting documentation/tasks.

## Packaging Goals

1. Ship reproducible binaries for the Rust CLI (`axion-cli`) and bundle the standard library (examples, schemas, UI build).
2. Provide first-class installers for Windows, macOS, and Linux, plus a portable archive for air‑gapped environments.
3. Expose update mechanisms so operators can stay on the latest release with minimal friction.
4. Make presets (Metasploit, OWASP ZAP, sqlmap, etc.) discoverable and updatable without rebuilding the core runtime.

## Deliverables

| Deliverable | Description | Dependencies |
|-------------|-------------|--------------|
| `axion-cli` binary release | Build optimized binaries for `x86_64` Windows/Linux/macOS. Publish artefacts to the release page. | GitHub Actions runners, Rust cross compilation toolchains |
| Standard library bundle | Tarball/zip containing `examples/`, `docs/`, `schemas/`, `ui/react-flow-prototype/dist/`. | UI build pipeline |
| Installer scripts | OS-specific setup wrappers (MSI/pkg/deb/rpm/tar). | Packaging toolchains (WiX, pkgbuild, cargo-deb, cargo-rpm) |
| Update channel | Command (`axion update`) or docs for `winget`, `brew`, `apt`, `dnf`. | Package repositories |
| Preset registry | Directory structure and manifest format for first-party and community tool definitions. | SDK schema, publish workflow |

## Platform Notes

### Windows

- Build using GitHub Actions `windows-latest` runner, targeting `x86_64-pc-windows-msvc`.
- Package with WiX or Inno Setup: install binaries under `%ProgramFiles%\Axion\bin`, add install root to PATH, register file association for `.ax` (optional).
- Provide PowerShell bootstrap (`install.ps1`) that downloads the MSI for unattended installs (fallback when GUI not available).
- Offer a Chocolatey manifest once the MSI is stable.

### macOS

- Build target `x86_64-apple-darwin` and `aarch64-apple-darwin` via cross toolchains.
- Produce a `.pkg` installer (via `pkgbuild`/`productbuild`) that installs into `/usr/local/axion` and symlinks `axion` to `/usr/local/bin`.
- Ship a Homebrew tap (`brew install axion/tap/axion`) referencing the tarball artefact.

### Linux

- Use `cargo-deb` to produce `.deb` packages (`/usr/bin/axion`). Provide systemd unit templates for future automation.
- Use `cargo rpm` or `cargo generate-rpm` for `.rpm` packages.
- Publish a generic tarball (`scripts/package/linux/package.sh`) (`axion-<version>-linux-x86_64.tar.gz`) (`axion-<version>-linux-x86_64.tar.gz`) extracting to `/opt/axion`.
- Document dependencies (glibc, libssl, etc.) so operators can pre-install prerequisites.

### Portable Archive

- Create a cross-platform ZIP with binaries and support files under `axion/`. Include a launcher script (`axion.cmd`, `axion.sh`) that resolves the embedded runtime without environment mutations.
- Useful for CI, containers, or air‑gapped installs.

## Preset & Tool Registry

1. Define manifest schema (`toolkit.json`) describing tool name, version, parameter schema path, binaries/resources, and documentation links.
2. Store official manifests under `presets/<tool>/<version>/` in the repository. Build pipeline copies them into release artefacts.
3. Runtime search order:
   - User overrides: `$AXION_HOME/tools.d/`
   - System presets: `/usr/share/axion/tools.d/` (or `%ProgramFiles%\Axion\tools.d`)
   - Built-ins bundled with CLI.
4. UI exposes registry for browsing and importing tool definitions. CLI provides `axion preset install <name>` to fetch from GitHub releases or a custom repository.

## Release Automation

1. GitHub Action triggered on tags `v*`:
   - Build binaries for each target (`cargo build --release`).
   - Run unit/integration tests.
   - Package installers/tarballs.
   - Upload artefacts to the GitHub Release.
   - Publish checksums.
2. Nightly snapshot (optional) pushing to a `nightly` channel for early testers.
3. Document fallback manual builds for contributors (mirroring what CI executes).

## Documentation & Support

- Update `docs/guide/installation.md` once installers exist (sections per platform, screenshots where relevant).
- Add a troubleshooting appendix (PATH issues, antivirus false positives, verifying checksums).
- Create quick-start video/gif demonstrating installer experience.
- Provide sample automation scripts (`install.ps1`, `install.sh`) calling package managers (`winget`, `brew`, `apt`, `dnf`). Link them from the UI settings modal.

## Outstanding Tasks

1. Provision packaging toolchains in CI (cache WiX, pkgbuild, cargo-deb/rpm).
2. Define CLI versioning policy and release schedule.
3. Finalise preset schema and bootstrap initial set (Metasploit, OWASP ZAP, sqlmap, nuclei, nikto).
4. Implement `axion preset` command (list/install/update/remove) with remote registry awareness.
5. Wire UI to preset registry (import existing scenario nodes, highlight missing binaries).
6. Draft user acceptance checklist (installer smoke tests, upgrade/downgrade paths).
7. Update roadmap and progress tracking as each deliverable lands.

---

This plan should be refined as packaging experiments begin. Keep the document up to date when major decisions (tool choice, directory layout, update channel) are finalised.
