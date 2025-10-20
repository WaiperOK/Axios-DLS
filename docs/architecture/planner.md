# Planner Concepts

The planner is the proposed component responsible for translating AxIR graphs into executable plans. While the current runtime executes steps sequentially, the planner will provide the groundwork for dependency analysis, optimisation, and distributed scheduling.

## Responsibilities

1. **Dependency Resolution** — Build a directed graph connecting step outputs (artifacts, variables) to downstream consumers.
2. **Capability Analysis** — Aggregate required capabilities (network access, filesystem writes, secret retrieval) to ensure the runtime satisfies policy constraints before execution.
3. **Optimisation** — Re-order or batch compatible operations when that does not alter observable semantics. Examples include grouping scans against the same target range or deduplicating imports.
4. **Plan Generation** — Produce an execution plan describing step order, parallelisation opportunities, retry policies, and error-handling directives.

## Plan Structure

| Field           | Description                                                         |
|-----------------|---------------------------------------------------------------------|
| `steps`         | Ordered list (or DAG) of plan nodes referencing AxIR node IDs.      |
| `dependencies`  | Mapping from step ID to prerequisite step IDs.                      |
| `capabilities`  | Union of capabilities required for the entire plan.                 |
| `strategy`      | Execution hints (serial, parallel batches, max concurrency).        |
| `policies`      | Timeouts, retry limits, escalation procedures.                      |

## Execution Policies

- **Retry semantics**: Configure per-step retry counts and backoff strategies; retain artifact history for failed attempts.
- **Timeouts**: Enforce global or per-capability deadlines to prevent runaway processes.
- **Conditional routing**: Allow future language features (conditional directives) to compile into alternative plan branches.

## Integration Points

- **CLI**: `axion plan` should expose the planner output in human-readable and machine-readable formats.
- **Agent/Server**: Remote executors can accept plans, schedule work across nodes, and stream artifact updates back to clients.
- **Audit Logging**: Persist plans and execution traces to facilitate compliance reviews and incident investigations.

The planner will evolve in tandem with AxIR. Early iterations can remain simple (serial plans with capability summaries) while providing the scaffolding for more advanced scheduling and automation.
