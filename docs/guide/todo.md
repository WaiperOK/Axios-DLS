# Backlog and Research Topics

This backlog groups significant enhancements and research items. Tags identify the primary focus area.

## Operational Foundations

- [ ] `[ops]` Author `CONTRIBUTING.md`, a code of conduct, and issue templates.
- [ ] `[ops]` Configure continuous integration (linting, tests, artifact uploads).
- [ ] `[ops]` Expose reproducible development environments via devcontainers.

## Language Evolution

- [ ] `[lang]` Finalise grammar v1.0, including alias syntax and error messaging.
- [ ] `[lang]` Implement a parser combinator library for detailed diagnostics.
- [ ] `[lang]` Add static analysis (`axion lint`) for undefined variables and unreachable steps.
- [ ] `[lang]` Define capability declarations to restrict tool usage per scenario.

## Runtime Enhancements

- [ ] `[runtime]` Introduce a dry-run executor that validates tool availability without invocation.
- [ ] `[runtime]` Support retry policies and backoff strategies for flaky tools.
- [ ] `[runtime]` Persist artifacts to pluggable backends (filesystem, S3-compatible storage).
- [ ] `[runtime]` Implement capability-based sandboxing for external processes.
- [ ] `[runtime]` Integrate secret managers (Vault, keyring) for secure parameter handling.

## Integrations and SDKs

- [ ] `[sdk]` Publish Rust and Python SDKs for consuming artifact data.
- [ ] `[sdk]` Provide schema definitions and client libraries for SARIF, SBOM, and OSV exports.
- [ ] `[sdk]` Offer sample adapters for Nmap, Masscan, OpenVAS, and custom scripts.

## Tooling and UX

- [ ] `[tooling]` Develop an artifact diff utility to compare scenario runs.
- [ ] `[tooling]` Investigate a graphical scenario editor (Tauri + React Flow).
- [ ] `[docs]` Build a documentation site (mdBook or Docusaurus) sourced from `docs/`.
- [ ] `[docs]` Expand example scenarios covering purple-team exercises and incident simulations.

## Observability and Operations

- [ ] `[ops]` Emit Prometheus metrics for execution duration, tool exit codes, and artifact counts.
- [ ] `[ops]` Add structured logging with trace identifiers for distributed execution.
- [ ] `[ops]` Provide deployment guidance for shared execution environments (agents, schedulers).

Items should be cross-referenced with GitHub issues when promoted to active development. Contributors are encouraged to propose specs or discovery documents before implementing complex features.
