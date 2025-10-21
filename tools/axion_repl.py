#!/usr/bin/env python3
"""Interactive Axion DSL console."""

from __future__ import annotations

import argparse
import os
try:
    import readline  # type: ignore
except ImportError:  # pragma: no cover
    readline = None
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

try:
    import axion_runner
except ImportError as exc:  # pragma: no cover
    raise SystemExit(f"Failed to import axion_runner: {exc}")


def write_temp(lines: list[str]) -> Path:
    tmp = tempfile.NamedTemporaryFile("w", delete=False, suffix=".ax")
    tmp.write("\n".join(lines) + "\n")
    tmp.close()
    return Path(tmp.name)


def render_execution(summary, execution, artifacts) -> None:
    print(axion_runner.format_summary(summary))
    print("\nExecution:")
    for step in execution:
        print(f"  - {step['type']} {step['name']}: {step['status']} ({step['message']})")
    failures = [step for step in execution if step["status"] == "failed"]
    if failures:
        print("\n[warn] some steps failed")
    if artifacts:
        print("\nArtifacts:")
        for artifact in artifacts:
            location = artifact.get("path") or "<memory>"
            print(f"  - {artifact['name']} ({artifact['kind']}) -> {location}")


def command_help() -> None:
    print("""Commands:
  :help               Show this message
  :show               Display current scenario buffer
  :clear              Reset the buffer
  :plan               Parse the buffer and print summary (honours overrides)
  :run                Execute the buffer and print artifacts
  :set KEY VALUE      Override variable (repeat to adjust multiple values)
  :unset KEY          Remove override
  :vars               Show active overrides
  :load PATH          Replace buffer with contents of PATH
  :save PATH          Write buffer to PATH
  :quit / :exit       Leave the console
""")


def axion_repl(initial: list[str] | None = None) -> None:
    buffer: list[str] = initial[:] if initial else []
    overrides: dict[str, str] = {}

    while True:
        try:
            line = input("axion> ")
        except EOFError:
            print()
            break
        except KeyboardInterrupt:
            print()
            continue

        stripped = line.strip()
        if not stripped:
            continue

        if stripped.startswith(":"):
            parts = stripped[1:].split()
            if not parts:
                continue
            cmd, *args = parts
            if cmd in {"quit", "exit"}:
                break
            if cmd == "help":
                command_help()
            elif cmd == "show":
                if not buffer:
                    print("[empty]")
                else:
                    for idx, content in enumerate(buffer, start=1):
                        print(f"{idx:03}: {content}")
            elif cmd == "clear":
                buffer.clear()
                print("[cleared]")
            elif cmd == "plan":
                if not buffer:
                    print("[warn] buffer is empty")
                    continue
                tmp = write_temp(buffer)
                try:
                    scenario = axion_runner.load_scenario(tmp)
                    summary = axion_runner.build_summary(scenario, overrides)
                    print(axion_runner.format_summary(summary))
                except Exception as exc:
                    print(f"[error] {exc}")
                finally:
                    tmp.unlink(missing_ok=True)
            elif cmd == "run":
                if not buffer:
                    print("[warn] buffer is empty")
                    continue
                tmp = write_temp(buffer)
                try:
                    scenario = axion_runner.load_scenario(tmp)
                    summary, execution, artifacts = axion_runner.execute_scenario(
                        scenario, json_mode=False, overrides=overrides
                    )
                    render_execution(summary, execution, artifacts)
                except Exception as exc:
                    print(f"[error] {exc}")
                finally:
                    tmp.unlink(missing_ok=True)
            elif cmd == "set" and len(args) >= 2:
                key = args[0]
                value = " ".join(args[1:])
                overrides[key] = value
                print(f"[var] {key} = {value}")
            elif cmd == "unset" and len(args) == 1:
                removed = overrides.pop(args[0], None)
                if removed is None:
                    print(f"[warn] override '{args[0]}' not defined")
                else:
                    print(f"[var] removed {args[0]}")
            elif cmd == "vars":
                if not overrides:
                    print("[var] (none)")
                else:
                    for k, v in overrides.items():
                        print(f"[var] {k} = {v}")
            elif cmd == "load" and len(args) == 1:
                path = Path(args[0]).expanduser()
                if not path.is_file():
                    print(f"[error] cannot read {path}")
                else:
                    buffer[:] = path.read_text().splitlines()
                    print(f"[load] {path} ({len(buffer)} lines)")
            elif cmd == "save" and len(args) == 1:
                path = Path(args[0]).expanduser()
                path.write_text("\n".join(buffer) + "\n")
                print(f"[save] {path}")
            else:
                print(f"[error] unknown command: {stripped}")
            continue

        buffer.append(line.rstrip("\n"))


def main() -> None:
    parser = argparse.ArgumentParser(description="Interactive Axion DSL console")
    parser.add_argument("path", nargs="?", help="Optional scenario to preload")
    args = parser.parse_args()

    initial: list[str] | None = None
    if args.path:
        scenario_path = Path(args.path)
        if not scenario_path.is_file():
            parser.error(f"cannot read {scenario_path}")
        initial = scenario_path.read_text().splitlines()

    print("Axion REPL. Type :help for commands.")
    axion_repl(initial)


if __name__ == "__main__":
    main()
