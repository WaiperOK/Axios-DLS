# Axion Intermediate Representation (AxIR)

AxIR is the planned intermediate representation for Axios scenarios. Although the current executor operates directly on parsed directives, AxIR will provide a normalised, analysis-friendly structure that enables optimisation, validation, and distributed scheduling.

## Goals

1. **Normalization** — Represent every directive as a node in a directed acyclic graph (DAG) with explicit inputs and outputs.
2. **Effect tracking** — Capture side effects (network scans, file writes, credential usage) to support policy enforcement and dry-run analysis.
3. **Scheduling** — Annotate dependencies to unlock parallel execution and remote dispatch while preserving determinism.
4. **Serialization** — Encode AxIR as JSON or protobuf for storage, versioning, and transmission to external executors.

## Proposed Node Schema

| Field       | Description                                                                 |
|-------------|-----------------------------------------------------------------------------|
| `id`        | Stable identifier derived from directive name and import lineage.           |
| `kind`      | Directive kind (`import`, `variable`, `asset_group`, `scan`, `script`, `report`). |
| `inputs`    | References to upstream artifact or variable nodes.                          |
| `outputs`   | Declared artifact or variable names produced by the node.                   |
| `params`    | Canonical parameter map (post interpolation).                               |
| `effects`   | Enumerated capabilities required (e.g., `network:scan`, `filesystem:write`). |
| `metadata`  | Optional annotations (owners, SLA, tags).                                    |

## Transformation Pipeline

1. **Parsing** — Convert source text into abstract syntax trees (existing parser).
2. **Validation** — Detect undefined variables, circular dependencies, and capability violations.
3. **Lowering** — Produce AxIR nodes with resolved paths, interpolated parameters, and defaulted values.
4. **Optimisation** — Collapse redundant nodes, batch compatible scans, or re-order steps that commute.
5. **Scheduling** — Generate execution plans (serial or parallel) tailored to the target runtime.

## Storage Options

- **In-memory**: Used by the CLI for planning and immediate execution.
- **Embedded database**: SQLite or PostgreSQL for long-lived plans, audit trails, and remote agents.
- **Graph databases**: Neo4j or similar for visualising relationships between assets, findings, and capabilities.

## Relationship to Runtime

AxIR will serve as the input to future executor backends. The current runtime can be viewed as operating on an implicit AxIR consisting of sequential nodes. Introducing an explicit representation will allow:

- Dry-run simulations independent of the execution engine.
- Incremental re-execution when only part of a plan changes.
- Formal verification of safety properties and compliance constraints.

Further details will be captured in dedicated ADRs once implementation work begins.
