# Type Considerations

Axios DSL is presently a string-oriented language. Nevertheless, the runtime and emerging tooling treat several concepts as implicitly typed. This document describes the current behaviour and outlines possible evolutions toward richer typing and effect systems.

## Current State

- **Variables** — Stored as strings. Consumers perform ad hoc parsing (e.g., interpreting `10.0.0.0/24` as a network range).
- **Asset properties** — Strings mapped by key. Shared conventions (such as `cidr`, `scope`, `owner`) rely on documentation rather than enforcement.
- **Scan parameters** — Strings that influence command invocation. Keys such as `flags` and `target` have conventional meaning but no type checking.
- **Artifacts** — JSON structures with predictable shapes (see `docs/book/appendix-artifacts.md`) that implicitly encode types (integers for ports, strings for severity).

## Validation Strategies

1. **Schema-driven linting** — Associate JSON Schema definitions with modules to validate variables and artifact transformations.
2. **Type annotations** — Allow optional annotations (`let cidr: network = "10.0.0.0/24"`) to trigger compile-time validation once supported by the parser.
3. **Capability declarations** — Specify allowable side effects (`scan discovery nmap!scanner`) to reason about required privileges.

## Planned Extensions

- **Primitive type library**: Networks, hostnames, URIs, durations, severity levels.
- **Collections**: Lists and maps with homogeneous element types for improved validation.
- **Effect system**: Distinguish between passive data processing and active operations (network scanning, file modification) to aid risk assessment.
- **Secret handling**: Introduce a `secret` type to ensure sensitive values are redacted from artifacts and logs.

All future type features will prioritise backward compatibility by treating annotations as optional hints until tooling matures to enforce them.
