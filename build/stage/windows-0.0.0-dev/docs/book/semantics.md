# Operational Semantics

The operational semantics of Axios describe how a parsed scenario is reduced to observable side effects. The reference executor evaluates directives sequentially, maintaining three mutable stores:

1. **Variable store**: string substitutions accessible to later steps.
2. **Artifact store**: structured representations of outputs keyed by artifact name.
3. **Execution trace**: ordered list of step outcomes containing status, messages, and diagnostic metadata.

## Evaluation Order

1. All `import` directives are resolved recursively prior to execution, producing a flattened scenario.
2. Directives are processed in source order. Each directive yields a `StepExecution` record with status (`completed`, `failed`, `skipped`, or `not_implemented`).
3. Failures halt neither execution nor reporting by default, but they do propagate through missing artifact references.

## Variable Substitution

- Variables introduced via `let` are string values. The executor resolves placeholders of the form `${variable_name}` in subsequent directives.
- Substitution applies to asset properties, scan parameters, script parameters, and report includes.
- Undefined variables trigger a failure in the referencing step.
- Substitution is single-pass: results are not re-interpreted for nested placeholders to prevent cyclic expansion.

## Asset Groups

- Asset groups contribute metadata without invoking external tools. They are recorded as `AssetGroupArtifact` instances.
- Because they are immutable after declaration, subsequent steps must refer back to the artifact to access properties; direct mutation is not supported.
- Asset groups yield `skipped` status to signal that no execution occurred, yet the artifact store was enriched.

## Scans

- For tools other than `nmap`, the executor performs a generic command invocation:
  - `tool` supplies the binary name.
  - `params.flags` and `params.args` are tokenised using POSIX shell splitting.
  - `params.target` is appended as a free-form argument.
  - `params.cwd` changes the working directory.
- Standard output, standard error, exit code, invocation details, and timing metadata are captured in a `Scan` artifact. Failures result in status `failed` with diagnostic text; success yields `completed`.
- For `nmap`, the executor enforces `-oX -` to force XML output. The XML is converted into a structured `ScanArtifacts` payload containing hosts, ports, service metadata, and enriched findings. This parsed structure feeds tabular reports.

## Scripts

- `script` directives execute arbitrary programs. The required parameter `run` is tokenised into executable and arguments.
- Optional `args` and `cwd` behave analogously to scan parameters.
- Outputs are stored in `ScriptArtifact` records with stdout, stderr, exit status, and timing data.
- Non-zero exits mark the step as `failed` while still preserving the artifact for forensic inspection.

## Reports

- Report directives reference existing artifact names. Missing artifacts trigger failure.
- The canonical `stdout` report streams a JSON payload describing all included artifacts, augmented with derived tabular summaries for scan findings.
- Reports emit `ReportArtifact` instances regardless of target; future backends may render to files, APIs, or ticketing systems.

## Error Handling

- Lexical and syntactic errors prevent a scenario from loading.
- Runtime failures mark individual steps and propagate through dependent reports when artifacts are absent.
- The executor never retries failed commands; idempotence must be ensured by the tool author.

## Determinism and Side Effects

- Scenarios are deterministic with respect to declared parameters and the behaviour of underlying tools. External side effects (such as network state) remain the responsibility of the operator.
- Randomisation must be explicit. Authors should model non-deterministic values via precomputed variables stored outside the scenario file to maintain reproducibility.

## Artifact Persistence

- Artifacts are written under the `artifacts/` directory by default. Filenames derive from sanitized artifact labels (`a-z`, `A-Z`, digits, underscore, hyphen, dot).
- The executor attempts to create the directory lazily and degrades gracefully if the filesystem is unavailable, emitting warnings without halting execution.

These semantics establish a predictable, inspectable execution model suitable for automation pipelines, collaborative analysis, and future formal verification efforts.
