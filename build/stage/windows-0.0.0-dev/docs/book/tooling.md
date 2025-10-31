# Toolchain Integration

The Axios toolchain consists of the Rust workspace, the `axion-cli` binary, and companion assets that package reusable scenario fragments. This chapter documents how practitioners interact with the toolchain to plan, execute, and extend Axios scenarios.

## Command-Line Interface

The CLI exposes two primary commands. Both accept `--json` to render machine-readable output.

### `plan`

```
cargo run -p axion-cli -- plan examples/demo.ax
```

- Parses the scenario (including imports) and emits the execution plan.
- Outputs a human-readable summary listing variables, asset groups, scans, scripts, and reports.
- When `--json` is provided, the summary is emitted as structured JSON suitable for automation.

### `run`

```
cargo run -p axion-cli -- run examples/demo.ax
```

- Performs the same planning pass as `plan`.
- Executes the scenario via the runtime, producing artifacts under `artifacts/`.
- Streams the execution report, and optionally a JSON payload that contains both the report and artifact metadata.
- Does not currently support selective execution; partial runs can be emulated by editing the scenario to include only the desired steps.

## Scenario Modules

- Modules are plain `.ax` files stored under directories such as `examples/modules/`.
- Reusable constructs (for example, standard asset group definitions or vulnerability scanning pipelines) should be encapsulated in modules and imported via relative paths.
- Module authors should provide accompanying documentation or inline comments describing expected variables and outputs.

## Workspace Layout

- `core`: houses the parser, runtime, and artifact definitions.
- `cli`: compiles the command-line interface.
- `examples`: contains ready-to-run scenarios demonstrating language features.
- `docs`: hosts the manual, guides, reference material, and architectural records.

Practitioners extending the runtime should work inside the workspace to benefit from shared dependencies and consistent build tooling.

## Build and Test

```
cargo fmt
cargo clippy --all-targets --all-features
cargo test --workspace
```

- The project inherits standard Rust tooling. Although the executor currently performs I/O-heavy tasks, unit tests focus on parser behaviour, variable substitution, and artifact construction.
- Integration tests should stub external tools to avoid destructive scans.

## Artifact Management

- Generated artifacts default to the `artifacts/` directory relative to the invocation location.
- Clean up artifacts between runs to avoid mixing results from distinct engagements. The provided `.gitignore` excludes the directory from version control to prevent accidental leakage.
- Downstream systems can ingest artifacts by reading the JSON files directly or by consuming the CLI `--json` output.

## Extending the Toolchain

1. **New scan adapters**: Implement additional parsing in the runtime by matching on `scan.tool`. Ensure artifacts remain backward compatible and document new fields.
2. **Alternative reporters**: Extend the `report` handler to recognise new targets (e.g., `report sarif`). Emit structured artifacts and update documentation accordingly.
3. **Custom SDKs**: Build language bindings around the JSON artifact schema to integrate Axios with orchestrators or data warehouses.
4. **Packaging**: Distribute pre-defined modules via Git submodules or package registries. Provide semantic versioning to signal compatibility with scenario constructs.

## Automation and CI/CD

- Use `plan` in pull request validation to ensure scenarios remain syntactically valid.
- Execute `run` in controlled staging environments to exercise complete pipelines. Capture artifacts as build artifacts for inspection.
- Gate merges on deterministic artifact hashes or structured diffing to detect unintentional behavioural changes in scenarios.

The toolchain is deliberately lightweight: a standard Rust toolchain and access to the requisite external binaries suffice. As the language evolves, additional tooling—such as language servers or web dashboards—can be layered without altering the core workflow.
