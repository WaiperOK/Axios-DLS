# Language Fundamentals

Axios scenarios are plain-text files encoded in UTF-8 and composed of ordered directives. The executor reads the file, resolves imports, and evaluates each step in sequence. This chapter introduces the structural elements that appear in any non-trivial scenario.

## File Structure

```
#!/usr/bin/env axion

import modules/web.ax

let inventory_root = "/opt/targets"

asset_group perimeter {
  scope = "external"
  location = "${inventory_root}/external.csv"
}

scan discovery {
  tool = "nmap"
  params.target = "perimeter"
  params.flags = "-sV -T4"
  output = "findings_discovery"
}

report stdout {
  include findings_discovery
}
```

Key observations:

- The optional shebang line allows scenarios to be executed directly on Unix-like systems.
- Blank lines and comments beginning with `#` improve readability and are ignored by the parser.
- Each directive introduces a step that contributes variables, artifacts, or reports to the execution.

## Directive Categories

Axios defines eight directive families:

| Directive      | Purpose                                                                    |
|----------------|----------------------------------------------------------------------------|
| `import`       | Inline another scenario file, eliminating duplicate definitions.          |
| `let`          | Define a typed variable (string, number, boolean, array, object) with optional interpolation. |
| `asset_group`  | Declare a set of assets (hosts, services, identities) and metadata.        |
| `scan`         | Invoke an external reconnaissance or assessment tool.                      |
| `script`       | Execute arbitrary automation such as exploit scripts or enrichment logic.  |
| `if`           | Conditionally execute nested steps based on boolean or comparison expressions. |
| `for`          | Iterate over arrays (or single values) and run a body for each element.    |
| `report`       | Collate artifacts into structured output for analysts or downstream tools. |

Each directive expands to a strongly typed structure in the runtime. The order of directives matters: variables must be defined before they are interpolated, and assets typically precede scans that depend on them.

## Identifiers and Naming

- Scenario identifiers (names of asset groups, scans, scripts, and reports) consist of ASCII letters, digits, hyphen, and underscore.
- An identifier becomes part of artifact labels; therefore, stable naming is essential for reproducibility and downstream automation.
- Variable names share the same character set and are case sensitive.

## Imports and Composition

`import <relative/path.ax>` statements appear at any point in the file. The executor canonicalises paths, detects cycles, and merges imported steps into the parent scenario. Variables declared in imported files are visible to subsequent steps. When naming collisions occur, last-one-wins semantics apply; style guides should discourage shadowing unless deliberate.

## Asset Groups

Asset groups capture structured metadata keyed by property name. Within reports, the executor merges asset properties with scan findings to improve situational awareness. Asset properties may contain variable interpolations, enabling parameterised inventories.

```
asset_group cloud_edge {
  provider = "aws"
  region = "${aws_region}"
  cidr = "198.51.100.0/24"
}
```

Asset groups themselves are not executed; they materialise as stored artifacts that other steps read or reference.

## Scans and Scripts

Both `scan` and `script` directives describe executable interactions with the environment.

```
scan port_survey {
  tool = "nmap"
  params.target = "198.51.100.23"
  params.flags = "-sS -sV --top-ports 1000"
  output = "survey_ports"
}

script fingerprint_web {
  params.run = "python3 tools/fingerprint.py"
  params.args = "--url https://198.51.100.23"
  output = "fingerprint_web"
}
```

- `tool` identifies the binary to invoke. For the reference executor, `nmap` receives specialised parsing; all other tools follow generic command execution.
- `params.*` entries are flattened into string-keyed arguments. Reserved keys such as `target`, `flags`, `args`, and `cwd` receive dedicated handling.
- `output` optionally overrides the artifact name used to store results.
- All parameters support `${variable}` interpolation.

## Control Flow

Control-flow directives allow scenarios to branch and repeat work without leaving the DSL.

```
let scan_enabled = true
let hosts = ["10.0.0.10", "10.0.0.11"]

if scan_enabled {
  for host in hosts {
    scan host_sweep {
      tool = "nmap"
      params.target = "${host}"
      params.flags = "-sV -Pn"
      output = "sweep_${host}"
    }
  }
} else {
  script notify_skip {
    run printf
    args "scan disabled"
    output skip_notice
  }
}
```

- `if` expressions accept boolean literals, variables, negation (`!expr`), and equality/inequality checks (`left == right`, `left != right`). `else` or `else if` branches are optional but must follow the closing brace of the preceding block.
- `for` loops expect an array literal (`["a", "b"]`) or a variable containing an array or string. The loop variable is rebound on each iteration and restored to its previous value afterwards.
- Steps nested inside control-flow blocks may declare additional variables, run scans, or emit reports. Failures inside one branch do not prevent subsequent top-level steps from executing.

## Reporting

Reports collect previously generated artifacts by name and render structured summaries. The canonical `stdout` report emits JSON and, for scan artifacts, ASCII tables.

```
report stdout {
  include survey_ports
  include fingerprint_web
}
```

Additional report targets may export artifacts to files, APIs, or dashboards as the runtime evolves. Reports fail if a referenced artifact is missing, ensuring early detection of inconsistent pipelines.
