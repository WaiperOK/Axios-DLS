# Typed Values

Axios variables now carry structured values rather than raw strings. The parser normalises literals into the following enum:

| Literal        | Example                                              | Notes                                                                 |
|----------------|------------------------------------------------------|-----------------------------------------------------------------------|
| `String`       | `"hello"` or `hello`                                 | Unquoted tokens fall back to strings.                                |
| `Number`       | `42`, `3.14`, `-10`                                  | Parsed as `f64`; the executor preserves integer-looking formats.     |
| `Boolean`      | `true`, `false`                                      | Case sensitive.                                                       |
| `Array`        | `[1, 2, "three"]`                                    | Elements may mix types.                                               |
| `Object`       | `{ host: "app", ports: [80, 443] }`                  | Keys may be bare identifiers or quoted strings; values follow any rule above. |

`let` declarations, CLI overrides (`--var KEY=VALUE`), and module imports all share this literal syntax. Nested structures are resolved recursively, so complex configuration can remain in a single variable.

## Literal Syntax Cheatsheet

- Arrays and objects must close their brackets; trailing commas are not supported.
- Object keys accept `[A-Za-z0-9_]+` or quoted strings (`"Content-Type"`).
- Single or double quotes work interchangeably for string literals.
- Numbers allow leading signs and decimal points; anything that fails numeric parsing becomes a string.

## Interpolation Semantics

During execution the runtime holds variables in a `HashMap<String, LiteralValue>`. When a literal is interpolated inside a string (`${name}`):

1. The variable is looked up after all prior `let` directives and overrides have resolved.
2. Structured values are serialised: numbers retain their canonical representation, arrays and objects render as JSON.
3. Missing variables trigger a runtime error.

When variables are reused programmatically (e.g., expanding a list of scan targets) the executor operates on the underlying structured type instead of the string form. This enables steps to consume arrays or maps without manual parsing.

## Collections in Practice

```
let scan_targets = ["192.0.2.10", "198.51.100.23"]
let metadata = { owner: "blue", priority: 2, sensitive: false }
```

`scan_targets` remains an array throughout execution. Reports or scripts can inspect the list directly, and interpolating `${scan_targets}` inside text produces `["192.0.2.10","198.51.100.23"]`.

## Roadmap

Future releases will layer validation and annotations on top of the existing literal engine:

1. **Schema-driven linting** to validate parameters before execution.
2. **Optional type hints** (`let cidr: network = ...`) to guide tooling.
3. **Secret-aware literals** that enforce redaction and safe storage.
4. **Capability declarations** for effect tracking (network scans, file writes).

These additions will remain backwards compatible by treating hints as opt-in metadata until full enforcement is available.
