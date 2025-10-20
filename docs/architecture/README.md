# Architecture Notes

These documents capture internal design considerations for the Axios runtime and its supporting tooling. They emphasise stability, extensibility, and the interfaces required for future distribution.

- [Axion Intermediate Representation](axir.md) — Planned structure for representing scenarios after parsing.
- [Planner](planner.md) — Concepts for building execution plans, dependency graphs, and optimisation passes.
- [Runtime](runtime.md) — Current implementation details and proposals for parallelism, sandboxing, and observability.

Additional design records (storage backends, capability systems, distributed agents) will be added as the project matures. Architectural Decision Records (ADRs) reside under `docs/architecture/adr/` when formal proposals are ratified.
