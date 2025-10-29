# Axion UI Builder

This prototype wraps the Axion DSL in an interactive React Flow canvas so that scenarios can be modelled visually, exported back to text, and executed through the embedded CLI bridge.

## Canvas basics

- Use the toolbar in the top‑left corner to add `import`, `asset`, `scan`, `script`, and `report` nodes.
- Drag nodes to arrange the layout. Handles on the top and bottom of each card create connections. Invalid connections are rejected with a warning.
- Double‑click a card to switch into inline editing mode. The sidebar always mirrors the currently selected node.

## Sidebar editor

1. **Scenario textarea**: paste raw Axion DSL and click **Import DSL** to rebuild the graph. The parser matches node labels and inserts edges automatically where possible.
2. **Node editor**: when a node is selected the section glows, highlighting the values that will be written back into the DSL preview.
3. **DSL preview**: reflects the graph in real time. Use **Copy DSL** to paste the output back into repository scenarios.

## Execution loop

| Action | Result |
| --- | --- |
| **Run scenario** button | Serialises the graph to temporary Axion DSL and calls `cargo run -p axion-cli -- run … --json`. |
| **CLI panel** | Sends arbitrary CLI commands (Ctrl/Cmd + Enter). Use this to plan files, export schemas, or run existing scenarios. |
| **Last run** card | Displays parsed JSON output: summary text, available artifacts, and the raw engine report. |
| **Logs** | Every stdout / stderr line is captured. Click an entry to open a modal with the full message for easier copy & paste. |

When the CLI reports artifacts (e.g. generated reports or stored files) they are listed with optional kind/path metadata.

## Keyboard shortcuts

- Double click – edit node inline.
- `Del` – remove the selected node or edge.
- `Ctrl/Cmd + Z` – undo (toolbar buttons are also provided).
- `Ctrl/Cmd + Y` – redo.
- `Ctrl/Cmd + Enter` – submit a CLI command.
- Right mouse button drag – pan; scroll wheel – zoom; `Shift + scroll` – horizontal pan.

## Troubleshooting

- If `Run` fails with import errors, confirm paths are relative to the temporary scenario (`examples/.ui-run-*.ax`). Using module imports from `examples/modules` keeps references valid.
- The dev server always invokes the CLI through Cargo to pick up the latest Rust changes. To opt into the compiled binary set `AXION_USE_BINARY=1` before launching Vite.

## Next steps

The UI is a prototype. Future work includes streaming logs, richer validation, tool discovery, and a connection between the graph and the secrets manager described in `docs/proposals/secrets-management.md`.
