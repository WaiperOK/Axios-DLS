# Axion DSL Roadmap (Work Queue)

The following tasks are ordered by priority. Complete each section before moving to the next unless dependencies are satisfied. This checklist is the authoritative work queue.

## 1. Typed Values & Data Structures
- [x] Extend `let` to accept typed literals (string, number, boolean, list, map).
- [x] Update parser to normalise literals into a type-aware AST.
- [x] Allow step parameters to consume typed values (e.g., expand lists for scans, emit structured JSON).
- [x] Document type system, literal syntax, and interpolation semantics.

## 2. Control Flow (Conditions & Loops)
- [x] Introduce `if`/`else` constructs with boolean expressions.
- [x] Introduce `for` loops over lists (e.g., `for target in targets { ... }`).
- [x] Implement runtime execution model (branching, repeated steps).
- [x] Provide examples and guidance for conditional execution.

## 3. Tool SDK & Parameter Validation
- [x] Define declarative schemas for built-in tools (nmap, gobuster, script, etc.).
- [x] Validate parameters during `plan`, emit actionable diagnostics.
- [x] Document the SDK format for third-party tool integrations.

## 4. Secrets Management
- [x] Support secrets blocks (e.g., `.env`, external vaults).
- [x] Ensure secrets are never printed in logs/artifacts.
- [x] Document recommended patterns for secure execution.

## 5. Reporting Backends
- [x] Add `report html` backend (template-based).
- [x] Add `report markdown` backend.
- [x] Add `report sarif` backend for pipeline integration.
- [x] Provide configuration knobs for report customisation.

## 6. UI Enhancements (React Flow Prototype → Full Editor)
- [x] Node toolbar: add/remove asset/scan/report/script blocks.
- [x] Inline editing: double-click to edit labels and parameters.
- [x] Edge management: delete selected edges, enforce valid connections.
- [x] Module awareness: visualise imports, allow module composition.
- [x] Export/import typed & control-flow enriched DSL.
- [x] Settings surface: persist preferences/profile, surface shortcuts/docs inline.
- [x] Log centre: stream output, collapse noisy lines, expose artifacts/summary inline.
- [ ] Polish UX copy (remove placeholder operator text, provide contextual hints).

## 7. Distribution & Tooling
- [ ] Provide an installer/package for the Axion language toolchain (CLI + runtime).
- [ ] Document installation & upgrade workflows across Windows/Linux/macOS.
- [ ] Offer presets for additional tools (Metasploit, OWASP ZAP, sqlmap, etc.) with validated schemas.
- [ ] Allow operators to register custom tools/modules via the SDK and UI.
- [ ] Enable discovery of external tool repositories (Git, local folder) from the UI.

## 8. Documentation Portal
- [ ] Serve documentation directly inside the UI (embedded Markdown renderer/static site).
- [ ] Refresh guides for secrets management and advanced DSL constructs.
- [ ] Add a “Getting Started” walkthrough tying CLI install, presets, and UI usage.

## 9. Intelligence & Automation
- [ ] Research LLM/ML assistants to orchestrate scan → attack → deep analysis flows.
- [ ] Design operator-in-the-loop UX for suggested actions and validation.
- [ ] Prototype API hooks for model integration (scenario generation, result triage, summarisation).

Maintain this file as the single source of truth for future development. Update status on completion of each bullet.
