# Specification Notes

This document highlights normative rules that govern Axios DSL. It supplements the comprehensive grammar and semantics described in the manual by emphasising author responsibilities and compatibility guarantees.

## Source Form

- Files **must** be UTF-8 encoded. Editors introducing byte order marks or legacy encodings are unsupported.
- Lines **may** end with either LF or CRLF; the parser tolerates both.
- Comments begin with `#` or `//` and extend to the end of the line.

## Directives

- Directive keywords are reserved: `import`, `let`, `asset_group`, `group`, `scan`, `script`, `report`, `if`, `else`, `for`.
- Identifiers **must** match `[A-Za-z0-9_-]+`. The parser rejects identifiers starting with digits for variables.
- Imports **must** resolve to accessible files; cyclic imports are ignored after the first inclusion to prevent infinite recursion.

## Variables

- Variables store typed literals (string, number, boolean, array, object) as normalised values.
- Interpolation resolves variables at runtime and renders them as strings; arrays and objects are encoded as JSON.
- Undefined variables raise runtime errors when encountered.
- Nested interpolation or expression evaluation is not supported; authors **must** precompute complex values externally.

## Asset Groups

- Properties are stored as opaque strings. Consumers **must not** rely on implicit typing (for example, numeric comparisons) without explicit parsing.
- Duplicate keys within a single asset group are overwritten by the last definition; avoid reuse unless intentional.

## Scans and Scripts

- Step names and artifact aliases share the same namespace. Authors should ensure that artifact names referenced in reports are unique.\n- The executor does not sandbox external tools. Scenarios **must** run on hardened hosts.\n- Timeouts and retries are currently absent; scripts requiring resilience should handle retries internally.\n- The CLI planner validates builtin tools (e.g., 
map requires 	arget); diagnostics are emitted before execution.

## Control Flow

- `if <expr> { ... }` evaluates boolean expressions. Supported forms include literals (`true`/`false`), boolean variables, logical negation (`!expr`), and equality/inequality comparisons (`a == b`, `a != b`) between literals or variables. `else` and `else if <expr>` clauses are optional; only the matching branch executes.
- `for <name> in <iterable> { ... }` iterates over arrays or single values. `<iterable>` accepts literals (e.g., `["a", "b"]`) or variables containing arrays or strings. Each iteration binds `<name>` to the current `LiteralValue`, executes the loop body, and restores any previously defined value for `<name>` after the loop completes.
- Steps nested inside control-flow blocks behave identically to top-level directives: they may import modules, declare variables, or emit artifacts. Failures within a branch or iteration do not abort subsequent steps unless explicitly coded.

## Reports

- Reports accept `report <name> [using <format>] { ... }`. When `using` is omitted the executor infers the format from `<name>` (e.g., `report stdout { ... }`).
- Supported formats: `stdout` (JSON emitted to console), `html` (static file under `artifacts/reports/<name>.html`), `markdown` (portable notes in Markdown), and `sarif` (SARIF v2.1.0 for findings exchange).
- Inside the block, each `include <artifact>` attaches an existing artifact. Optional `output "<path>"` overrides the default file location for file-based formats.
- `option <key> "<value>"` customises rendering. Recognised keys: `title` (HTML/Markdown heading), `tool_name`/`tool_version`/`tool_uri` (SARIF metadata), and `severity_threshold` (minimum severity included in SARIF output). Unrecognised keys are preserved in the emitted artifact for downstream consumers.
- Reports fail when includes reference missing artifacts. This behaviour is deliberate to expose broken pipelines.

## Evolution Policy

- New directive types or keywords require at least one preview release and accompanying migration guidance.
- Optional parameters may be added to existing directives provided that defaults preserve prior behaviour.
- Deprecations follow a two-release policy: marked deprecated in one release, removed in the next with clear warnings.

For a full grammar, consult `docs/book/appendix-grammar.md`. Runtime specifics and artifact schemas are defined in `docs/book/semantics.md` and `docs/book/appendix-artifacts.md`.


