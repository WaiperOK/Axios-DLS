# SDK Overview

Axios SDKs enable external systems to consume artifacts, manage scenarios, and integrate runtime capabilities into broader platforms. This document summarises planned language bindings and extension points.

## Target Languages

- **Rust** — Native bindings for embedding the executor or analysing artifacts within Rust applications. Will expose strongly typed structs mirroring `axion-core`.
- **Python** — Convenience API for data science workflows, orchestration frameworks, and notebook-driven analysis. Distribution via PyPI using PyO3 bindings.
- **Go** — Lightweight client for integrating with infrastructure automation and cloud-native tooling. Likely implemented using cgo or generated bindings.

## Functional Areas

| Module   | Purpose                                                                   |
|----------|---------------------------------------------------------------------------|
| `artifacts` | Parse and validate artifacts (`AssetGroup`, `Scan`, `Script`, `Report`). |
| `execution` | Invoke the CLI or embedded executor, stream execution events, manage artifacts. |
| `schema`    | Provide JSON Schema definitions and helper validators.                |
| `capabilities` | Describe required privileges and perform policy checks prior to execution. |

### Built-in Tool Schemas

`axion-core` exposes lightweight parameter schemas for first-party integrations. These are exercised automatically by `axion plan` and should also be consumed by SDK clients to provide consistent diagnostics.

| Tool      | Required parameters       | Optional parameters           | Notes |
|-----------|---------------------------|-------------------------------|-------|
| `nmap`    | `target`                  | `flags`                       | Errors if `target` is empty or missing; additional keys trigger warnings. |
| `gobuster`| `target`, `args`          | `flags`, `wordlist`, `mode`   | Ensures command arguments are provided; extra keys emit warnings. |
| `script`  | `run`                     | `args`, `cwd`                 | Validates that `run` is non-empty and quoted correctly. |

The `axion_core::builtin_tool_schemas()` function returns these definitions (serialised with Serde) so SDK clients can hydrate them into JSON Schema or other validation frameworks.

### Secret metadata

Scenarios may declare secrets via the `secret` directive (`from env`, `from file`, `from vault`). During execution the CLI and embedded runtime resolve `${secret:...}` placeholders using an in-memory `SecretStore` that automatically masks values in logs and artifacts.

- Override values at runtime with `axion run scenario.ax --secret alias.field=value`. Each flag maps to the alias defined inside the `secret` block (e.g., `db_creds.username`).
- `axion plan` performs structural checks: missing env mappings, empty file paths, or unknown providers produce diagnostics so SDK integrations can present actionable UI.
- SDKs should surface `SecretSummary` metadata (name, provider) to editor integrations so they can prompt for secret wiring alongside tool parameter schemas.

The SDK `schema` module will expose both tool schemas and secret descriptors as serialisable metadata (e.g., JSON Schema) so external planners can extend or override behaviour while retaining compatibility with the CLI.

### Report outputs

`ReportArtifact` now includes a `format` string and optional `output_path`. File-backed formats (`html`, `markdown`) populate `output_path` with the resolved filesystem location, while `stdout` leaves it unset. SDK consumers should respect the format to decide how to render the artifact and treat unknown formats as opaque blobs.

## Packaging Guidelines

- SDKs must surface the artifact schema version to detect compatibility issues.
- Generated code should treat unknown fields as forward-compatible extensions.
- Authentication and secrets management are delegated to the caller; future releases may offer helpers that integrate with Vault or cloud key stores.

## Roadmap

1. Publish Rust crate with artifact parsing and execution helpers.
2. Release Python bindings focused on artifact analytics and CLI wrappers.
3. Prototype Go client for remote execution services.
4. Document examples showcasing integration with CI pipelines, dashboards, and vulnerability management systems.

Updates to the SDKs will be tracked in dedicated changelogs and aligned with the main project versioning scheme.


### UI Integration Notes

- The React prototype under `ui/` can load schema data by running `axion schema --format json` during build/startup and caching the resulting bundle (e.g., place it under `src/assets/tool-schemas.json`).
- Watch for updates: the bundle carries a `version` field; regenerate when it changes or on CLI upgrades.
- For live editing, invoke the CLI programmatically via a dev server or call `axion_core::builtin_tool_schema_bundle()` from a Rust backend.
