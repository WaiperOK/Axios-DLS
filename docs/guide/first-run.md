# First Run Guide

This guide walks through installing the Axios DSL workspace, executing the sample scenario, and examining generated artifacts. Follow the steps sequentially on a workstation with Rust tooling installed.

## Prerequisites

- Rust toolchain (`rustup`, `cargo`, `rustc`) version 1.78 or newer.
- Access to external binaries required by scenarios (for example `nmap` for discovery scans).
- Network connectivity appropriate for the intended targets.

## Build the Workspace

```
git clone https://github.com/WaiperOK/Axios-DLS.git
cd Axios-DLS
cargo build --workspace
```

- The command produces `target/debug/axion-cli`.
- Ensure the build output is free of warnings to catch dependency or feature mismatches early.

## Execute the Introductory Scenario

```
cargo run -p axion-cli -- run examples/hello.ax
```

- The CLI resolves imports, evaluates the scenario, and prints an execution report.
- The `stdout` report emits JSON containing all referenced artifacts.

Expected console excerpt:

```
Execution results:
  - [completed] hello (Report)

{ ... JSON summary omitted for brevity ... }
```

## Inspect Artifacts

Artifacts are emitted under `artifacts/`. Verify that the directory contains the report generated in the previous step.

```
ls artifacts
cat artifacts/report_stdout.json
```

- Each artifact file is indented JSON. Use standard tooling (`jq`, `python -m json.tool`) for inspection.
- Delete the directory between runs if you want to avoid mixing results from different scenarios.

## Run the Demonstration Scenario

```
cargo run -p axion-cli -- run examples/demo.ax --json > demo-run.json
```

- The `--json` flag emits a machine-readable execution summary and artifact listing.
- Review the generated file to understand how scans, scripts, and reports interact.

## Next Steps

- Study [`docs/book/fundamentals.md`](../book/fundamentals.md) for a deeper explanation of directives.
- Adapt `examples/demo.ax` by introducing custom asset groups or scripts relevant to your environment.
- Optionally, integrate the command into a CI job to validate that scenarios remain executable across commits.
