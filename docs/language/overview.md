# Conceptual Overview

Axios DSL models security engagements as ordered sequences of directives. Each directive either records context, invokes tooling, or collates results. The language emphasises clarity and reproducibility over compact syntax.

## Minimal Scenario

```
let cidr = "10.0.0.0/24"

asset_group perimeter {
  scope external
  cidr ${cidr}
}

scan discovery nmap {
  flags -sV -Pn
  target ${cidr}
} -> discovery_findings

script fingerprint {
  run python3 tools/fingerprint.py
  args --target ${cidr}
} -> fingerprint_results

report stdout {
  include discovery_findings
  include fingerprint_results
}
```

## Directive Classes

| Directive      | Role                                                                 |
|----------------|----------------------------------------------------------------------|
| `let`          | Define typed variables (string, number, boolean, array, object) for reuse and interpolation. |
| `asset_group`  | Document assets, scopes, or metadata relevant to the assessment.     |
| `scan`         | Execute reconnaissance tools; special handling provided for Nmap.    |
| `script`       | Run arbitrary automation (exploitation scripts, enrichment tasks).   |
| `if`           | Evaluate a boolean expression and execute steps conditionally (optional `else`). |
| `for`          | Iterate over arrays or single values, binding each element to a loop variable. |
| `report`       | Assemble artifacts into structured output for analysts.              |
| `import`       | Compose scenarios by inlining external files.                        |

## Control Flow Example

```
let targets = ["10.0.0.10", "10.0.0.11"]
let scan_enabled = true

if scan_enabled {
  for host in targets {
    scan host_sweep nmap {
      flags -sV -Pn
      target ${host}
    } -> sweep_${host}
  }
} else {
  report stdout {
    include noop_warning
  }
}
```

The loop binds each address in `targets` to `host` before executing the nested `scan`. Any step (including additional `if` blocks or `let` declarations) can appear inside control-flow blocks.

## Composition Patterns

- **Parameterised modules** - Encapsulate reusable logic in `.ax` files and expose required variables. Import them from scenario skeletons to adapt to new targets.
- **Artifact chaining** - Use script directives to post-process scan artifacts and emit new data structures (e.g., severity classification). Reports can then include both the raw and enriched artifacts.
- **Progressive enrichment** — Run fast scans first, feed results into targeted scripts, and consolidate everything via reports. Each stage remains auditable due to artifact retention.

## Versioning Philosophy

- The grammar evolves conservatively. Additions to syntax or directive semantics require backwards-compatible paths and clear migration guidance.
- Non-breaking enhancements (new parameters, report targets) should be documented with explicit default behaviour.
- Experimental features may be gated via annotations or opt-in flags before exiting incubation.

Understanding these concepts prepares practitioners to dive deeper into the [language specification](spec.md) and contribute to future design discussions.
