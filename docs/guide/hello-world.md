# Authoring a Minimal Scenario

This guide constructs a minimal yet complete Axios scenario to highlight each directive and its effect on execution. It assumes that you completed the [First Run Guide](first-run.md).

## Directory Setup

Create a workspace for experimentation and initialise an empty scenario file:

```
mkdir -p scenarios
cd scenarios
cat <<'EOF' > hello-world.ax
#!/usr/bin/env axion

let greeting = "Hello, Axios"

asset_group demo_hosts {
  scope lab
  description ${greeting} hosts prepared for validation
}

scan banner nmap {
  flags -sn
  target 192.0.2.10
} -> banner_scan

report stdout {
  include banner_scan
}
EOF
```

## Directive Breakdown

1. **Shebang** - Allows the scenario to be executed directly on Unix systems.
2. **Variable** (`let greeting = ...`) - Demonstrates interpolation inside an asset group property.
3. **Typed literals** - `let` accepts numbers, booleans, arrays, and maps (e.g., `let targets = ["10.0.0.1", "10.0.0.2"]`, `let metadata = { owner: "blue", priority: 2 }`). Values remain available for interpolation inside strings and parameters.
4. **Asset Group** (`asset_group demo_hosts`) - Documents the scope in which the scenario operates. Properties are expressed as whitespace-delimited key and value.
5. **Scan** (`scan banner nmap`) - Invokes `nmap -sn 192.0.2.10`. The closing `-> banner_scan` alias renames the resulting artifact.
6. **Report** - Emits a JSON payload (and optional ASCII table) to standard output that includes `banner_scan`.

### Adding Secrets

Sensitive inputs such as credentials or API tokens are declared with `secret` blocks. Values populate a runtime-only secret store and can be interpolated inside strings with `${secret:...}` placeholders.

```axion
secret http_creds from env {
  username = "HELLO_WORLD_USER"
  password = "HELLO_WORLD_PASS"
}

let auth_header = "Basic ${secret:http_creds.username}:${secret:http_creds.password}"
```

- `from env` maps friendly aliases (`username`, `password`) to environment variables.
- `from file "path/to/secret"` loads the file contents into the secret store.
- Any message that includes the resolved secret is automatically masked in the execution report.

At runtime you may override values without touching local env/files:

```
cargo run -p axion-cli -- run hello-world.ax \
  --secret http_creds.username=admin \
  --secret http_creds.password="$(pass show hello-world)"
```

Use `--secret` for secret substitutions, and `--var` for plain variables. Both flags accept multiple entries.

## Execution

```
cargo run -p axion-cli -- run scenarios/hello-world.ax --var greeting="Salutations"
```

The optional `--var KEY=VALUE` flag overrides any `let` declaration at runtime; in this example the greeting interpolated inside the asset group is replaced without editing the source scenario. Repeat the flag to adjust multiple variables.

- Observe the execution report and the JSON emitted by the `stdout` report.
- Review `artifacts/banner_scan.json` to inspect the raw data captured from `nmap`.

## Experimentation

- Modify the scan flags to collect service banners (`-sV`) and compare artifacts.
- Add a `script` directive that parses the scan artifact and classifies exposed services.
- Duplicate the report block as `report html_summary using html { ... }` to render a static HTML summary under `artifacts/reports/`.
- Emit complementary notes with `report notebook using markdown { ... }` to capture findings in a shareable `.md` file.
- Introduce additional reports (e.g., `report sarif`) once new backends are implemented.
- Wrap sections in `if` conditions or use `for target in [...]` loops to fan out scans across multiple hosts.

## Cleanup

Delete the `artifacts/` directory to reset the workspace before the next experiment:

```
rm -rf artifacts
```

This minimal scenario serves as a template for more elaborate automation. Gradually layer additional directives, imports, and reporting targets to encode your own assessment methodology.

For quick experiments without editing files, start the interactive console:

```bash
python tools/axion_repl.py
```

Use `:help` to list commands; buffers accept the same syntax demonstrated above.


