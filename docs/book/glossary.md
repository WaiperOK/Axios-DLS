# Glossary

**Artifact** — Structured JSON record capturing the outcome of a step (asset group, scan, script, report) together with metadata required for reproducibility.

**Asset Group** — Declarative collection of assets or scopes with associated properties. Serves as context for subsequent scans and reports.

**Axios Scenario** — Executable document composed of directives (`import`, `let`, `asset_group`, `scan`, `script`, `report`) that models a security engagement.

**Directive** — Single instruction within a scenario that the executor interprets. Directives are processed sequentially.

**Execution Outcome** — Pair of the execution report and all artifacts produced during a run.

**Execution Report** — Ordered list of `StepExecution` entries indicating status (completed, failed, skipped, not implemented) and diagnostic messages.

**Finding** — Individual record within a `ScanArtifact` representing a detected service, vulnerability, or observation. Contains severity, description, and evidence fields.

**Import** — Mechanism for composing scenarios by inlining external `.ax` files with cycle detection.

**Runtime** — Reference implementation (`axion-core`) responsible for evaluating scenarios and managing artifacts.

**Scan** — Directive that invokes an external reconnaissance tool, typically producing a `ScanArtifact`.

**Script** — Directive that executes arbitrary automation with controlled parameterisation, producing a `ScriptArtifact`.

**Variable** — Named string populated through `let` directives and interpolated into subsequent directives via `${name}` placeholders.
