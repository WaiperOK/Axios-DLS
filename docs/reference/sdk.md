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
