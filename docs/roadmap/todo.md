# Axion DSL Roadmap (Work Queue)

The following tasks are ordered by priority. Each section should be completed before moving to the next, unless dependencies are resolved explicitly. Use this checklist as the authoritative work queue.

## 1. Typed Values & Data Structures
- [x] Extend `let` to accept typed literals (string, number, boolean, list, map).
- [x] Update parser to normalise literals into a type-aware AST.
- [x] Allow step parameters to consume typed values (e.g., expand lists for scans, emit structured JSON).
- [ ] Document type system, literal syntax, and interpolation semantics.

## 2. Control Flow (Conditions & Loops)
- [ ] Introduce `if`/`else` constructs with boolean expressions.
- [ ] Introduce `for` loops over lists (e.g., `for target in targets { ... }`).
- [ ] Implement runtime execution model (branching, repeated steps).
- [ ] Provide examples and guidance for conditional execution.

## 3. Tool SDK & Parameter Validation
- [ ] Define declarative schemas for built-in tools (nmap, gobuster, script, etc.).
- [ ] Validate parameters during `plan`, emit actionable diagnostics.
- [ ] Document the SDK format for third-party tool integrations.

## 4. Secrets Management
- [ ] Support secrets blocks (e.g., `.env`, external vaults).
- [ ] Ensure secrets are never printed in logs/artifacts.
- [ ] Document recommended patterns for secure execution.

## 5. Reporting Backends
- [ ] Add `report html` backend (template-based).
- [ ] Add `report markdown` backend.
- [ ] Add `report sarif` backend for pipeline integration.
- [ ] Provide configuration knobs for report customisation.

## 6. UI Enhancements (React Flow Prototype → Full Editor)
- [ ] Node toolbar: add/remove asset/scan/report/script blocks.
- [ ] Inline editing: double-click to edit labels and parameters.
- [ ] Edge management: delete selected edges, enforce valid connections.
- [ ] Module awareness: visualise imports, allow module composition.
- [ ] Export/import typed & control-flow enriched DSL.

Maintain this file as the single source of truth for future development. Update status on completion of each bullet.***
