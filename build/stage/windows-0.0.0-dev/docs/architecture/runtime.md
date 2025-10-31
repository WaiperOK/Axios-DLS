# Runtime Architecture Notes

The current runtime (`core/src/runtime.rs`) executes scenarios sequentially. This document outlines its internal mechanics and sketches future enhancements required for scale, safety, and distributed operation.

## Present Implementation

- **Executor**: Creates an `artifacts/` directory, iterates over steps, and dispatches to handler functions (`process_variable`, `process_asset_group`, `process_scan`, `process_script`, `process_conditional`, `process_loop`, `process_report`).
- **Variable Store**: `HashMap<String, LiteralValue>` populated by `let` directives and CLI overrides; supports typed interpolation.
- **Artifact Store**: `HashMap<String, StoredArtifact>` keyed by artifact name, enabling downstream steps to reference prior outputs.
- **Control Flow**: `execute_steps` recurses into nested blocks, recording the outcome of conditions and the iteration count of loops.
- **Validation**: `validate_scenario` enforces builtin tool schemas (e.g., `nmap` requires `target`) during planning.
- **Reporting**: Generates `ExecutionReport` (status, messages) and returns artifacts to the CLI.
- **Nmap Specialisation**: Parses XML output into structured findings, enabling tables and machine-readable reports.

## Planned Enhancements

1. **Abstract Execution Environment** — Decouple process invocation from the host OS to enable sandboxing, containerisation, or remote agents.
2. **Parallelism** — Execute independent steps concurrently once dependency graphs are available from the planner.
3. **Capability Enforcement** — Introduce policy checks before executing steps requiring privileged operations (filesystem writes, network scans).
4. **Resilience** — Implement configurable retries, timeouts, and cancellation.
5. **Observability** — Emit structured logs, metrics (Prometheus/OpenTelemetry), and audit trails to track tool usage and outcomes.

## Component Diagram (Proposed)

- `PlanExecutor` — Accepts execution plans (future output of planner), orchestrates scheduling, and records telemetry.
- `ProcessAdapter` — Pluggable interface for running external commands (local process, container runtime, remote RPC).
- `ArtifactSink` — Writes artifacts to configured backends (filesystem, object storage, databases).
- `EventBus` — Streams step lifecycle events to subscribers (CLI, web UI, logging systems).

## Safety Considerations

- Provide strict defaults (no network or filesystem access) until capabilities are explicitly granted.
- Track provenance of artifacts, including command arguments and environment variables, for compliance.
- Support dry-run mode that validates tool availability and plan structure without invoking external commands.

## Integration Path

Upgrades should be incremental:

1. Refactor the existing executor behind trait-based abstractions.
2. Introduce optional concurrency for steps without dependencies.
3. Layer policy checks and sandboxing.
4. Expose gRPC/HTTP APIs for remote execution once stability is proven.

These notes will evolve into formal ADRs as implementation work progresses.


