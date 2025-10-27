# Runtime Architecture

The reference runtime, implemented in `core/src/runtime.rs`, materialises the operational semantics described earlier. It is intentionally monolithic at this stage to keep scheduling and state management transparent. Future releases may factor the executor into services or distributed components, but the foundational responsibilities will remain.

## Components

### Executor

- Creates the artifact store directory (`artifacts/`) on instantiation.
- Maintains:
  - An in-memory `HashMap<String, StoredArtifact>` keyed by artifact name.
  - The variable store (`HashMap<String, LiteralValue>`) populated by `let` directives and CLI overrides.
  - The execution trace (vector of `StepExecution` records).
- Provides `execute(&Scenario)` which iterates over flattened steps, dispatching to specialised handlers (variables, assets, scans, scripts, conditionals, loops, reports).

### StepOutcome

Internal utility that pairs a `StepExecution` with an optional `StoredArtifact`. This abstraction simplifies accumulation of artifacts and metrics across heterogeneous directives.

### Artifact Types

Defined in `core/src/artifact.rs`:

- `AssetGroupArtifact`: metadata map.
- `ScanArtifact` and `ScanArtifacts`: raw and parsed outputs of reconnaissance tools.
- `ScriptArtifact`: stdout, stderr, command invocation metadata.
- `ReportArtifact`: generated report payload including tables rendered for scans.
- `TableArtifact`: columnar representation used for ASCII rendering.

### ExecutionReport and ExecutionOutcome

- `ExecutionReport` is a serialisable trace of step statuses.
- `ExecutionOutcome` bundles the report with all persisted artifacts, enabling downstream consumers to reason about both control flow and side effects.

## Dispatch Flow

1. Skip `import` directives (already resolved).
2. Invoke `process_variable`, `process_asset_group`, `process_scan`, `process_script`, `process_conditional`, `process_loop`, or `process_report` depending on the step type.
3. For each stage, attach any generated artifact to the store using `StoredArtifact::name` as the key.
4. Recurse into nested blocks (`if` branches, `for` bodies) so that child steps share the same variable store and artifact registry.
5. After traversal completes, return the execution report and collected artifacts.

## Variable Resolution

`substitute_variables` performs streaming substitution with a single pass through the string. The behaviour is deterministic and emits descriptive errors for undefined variables or malformed placeholders. Both maps (`resolve_map`) and lists (`resolve_list`) delegate to this routine.

## Scan Specialisation

- Generic scan handling constructs a `std::process::Command` based on parameters, captures output, and writes an artifact JSON file. Invocation arguments are preserved to guarantee reproducibility.
- The Nmap specialisation parses XML output via `quick-xml` into domain-specific structures (hosts, addresses, ports, services, findings). This parsed representation simplifies reporting and downstream analytics.

## Script Execution

Script handlers closely mirror generic scans but enforce the presence of `run`. They tokenise the command line using `shell_words` to align with POSIX-style quoting, even on Windows targets.

## Control Flow Handling

- `process_conditional` evaluates boolean expressions (literals, variables, negation, equality/inequality) and records a `StepExecution` describing the outcome before executing the matching branch.
- `process_loop` resolves iterables to `LiteralValue` sequences, binds the loop variable per iteration, and restores any previous binding after completion.
- Both handlers delegate to `execute_steps`, so nested directives share the same artifact store, override map, and execution log as their parent context.

## Parameter Validation

- `validate_scenario` runs before planning to ensure builtin tool schemas are satisfied (for example, `nmap` requires `target`, `gobuster` requires both `target` and `args`).
- Diagnostics surface with severity (`error`/`warn`) and are emitted even in JSON output, allowing CI pipelines to fail early.

## Reporting Pipeline

- Includes are resolved to existing artifacts; missing references yield `failed` status.
- For scan artifacts, `build_table_from_scan` constructs a canonical seven-column table to support consistent comparisons across tools.
- Reports targeting `stdout` echo a pretty-printed JSON payload and the derived tables.

## Artifact Persistence

Artifacts are written through `write_artifact`, which sanitises labels, serialises JSON with indentation, and handles I/O errors gracefully. When persistence fails, the executor still returns in-memory artifacts so that the caller can act on the results.

## Extensibility Surface

Future runtime extensions should respect the following constraints:

- **Idempotent handlers**: repeated execution should not mutate external state beyond the invoked tools.
- **Explicit configuration**: new step types must declare their parameters explicitly to remain auditable.
- **Observable diagnostics**: every handler must emit structured success or failure messages.
- **Deterministic scheduling**: even when introducing parallel execution, ordering guarantees should be maintained or made explicit.

This architectural baseline emphasises clarity over throughput, enabling practitioners to audit every transformation from scenario to artifact.





