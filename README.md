# Axios DSL

Axios DSL is a domain-specific language and execution environment for modelling offensive security campaigns, repeatable attack simulations, and operational resilience drills. The project couples a compact declarative syntax with an extensible runtime capable of orchestrating external scanners, scripted tooling, and analytic post-processing.

## Why Axios DSL

- **Scenario centric**: Encode asset inventories, discovery plans, exploitation probes, and reporting pipelines as first-class language constructs.
- **Deterministic automation**: Formalise engagements so that reconnaissance, active scanning, and enrichment steps remain reproducible across operators and iterations.
- **Native observability**: Transform raw tool output into structured artifacts, tabular summaries, and final reports without ad hoc scripting.
- **Toolchain neutrality**: Bind to any executable or scriptable endpoint; ship with opinionated adapters for ubiquitous utilities such as Nmap.

## Getting Started

1. Install the toolchain (`cargo build --workspace`) to obtain the `axion-cli` binary.
2. Explore the introductory scenario at `examples/hello.ax` and execute a dry run:
   ```
   cargo run -p axion-cli -- plan examples/hello.ax
   ```
3. Progress to `examples/demo.ax` to observe multi-step coordination of asset groups, scanners, and custom scripts.
4. Study the full specification in the [Axios DSL Manual](docs/book/README.md) to understand syntax, execution semantics, and integration patterns.

## Documentation

- **Manual**: `docs/book/` provides the comprehensive reference, including language philosophy, grammar, semantics, runtime behaviour, and architectural rationale.
- **Task-oriented guides**: `docs/guide/` contains concise workflows for common activities such as onboarding, continuous assessment, and extending the toolkit.
- **Operational reference**: `docs/reference/` catalogues command-line flags, artifact schemas, and integration checklists.
- **Technical architecture**: `docs/architecture/` captures the internal design, component boundaries, and ongoing RFCs.

## Contributing

Contributions are welcomed via pull requests focusing on language evolution, runtime integrations, documentation, or ecosystem tooling. Please adhere to the Apache 2.0 licence as stated in `LICENSE`, follow Conventional Commits for change history, and accompany substantive alterations with corresponding documentation updates.

## License

Axios DSL is distributed under the Apache License, Version 2.0. See `LICENSE` for details.
