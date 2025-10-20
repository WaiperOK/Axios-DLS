# Specification Notes

This document highlights normative rules that govern Axios DSL. It supplements the comprehensive grammar and semantics described in the manual by emphasising author responsibilities and compatibility guarantees.

## Source Form

- Files **must** be UTF-8 encoded. Editors introducing byte order marks or legacy encodings are unsupported.
- Lines **may** end with either LF or CRLF; the parser tolerates both.
- Comments begin with `#` or `//` and extend to the end of the line.

## Directives

- Directive keywords are reserved: `import`, `let`, `asset_group`, `group`, `scan`, `script`, `report`.
- Identifiers **must** match `[A-Za-z0-9_-]+`. The parser rejects identifiers starting with digits for variables.
- Imports **must** resolve to accessible files; cyclic imports are ignored after the first inclusion to prevent infinite recursion.

## Variables

- Variables are string-valued. Future revisions may introduce richer types, but all interpolation currently yields strings.
- Undefined variables raise runtime errors when encountered.
- Nested interpolation or expression evaluation is not supported; authors **must** precompute complex values externally.

## Asset Groups

- Properties are stored as opaque strings. Consumers **must not** rely on implicit typing (for example, numeric comparisons) without explicit parsing.
- Duplicate keys within a single asset group are overwritten by the last definition; avoid reuse unless intentional.

## Scans and Scripts

- Step names and artifact aliases share the same namespace. Authors should ensure that artifact names referenced in reports are unique.
- The executor does not sandbox external tools. Scenarios **must** run on hardened hosts.
- Timeouts and retries are currently absent; scripts requiring resilience should handle retries internally.

## Reports

- Report targets are strings. The reference executor recognises `stdout`; additional targets will be introduced in backwards-compatible releases.
- Reports fail when includes reference missing artifacts. This behaviour is deliberate to expose broken pipelines.

## Evolution Policy

- New directive types or keywords require at least one preview release and accompanying migration guidance.
- Optional parameters may be added to existing directives provided that defaults preserve prior behaviour.
- Deprecations follow a two-release policy: marked deprecated in one release, removed in the next with clear warnings.

For a full grammar, consult `docs/book/appendix-grammar.md`. Runtime specifics and artifact schemas are defined in `docs/book/semantics.md` and `docs/book/appendix-artifacts.md`.
