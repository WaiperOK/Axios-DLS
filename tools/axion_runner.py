#!/usr/bin/env python3
"""
Lightweight Axion DSL runner implemented in Python.

Usage:
    python tools/axion_runner.py plan examples/demo.ax
    python tools/axion_runner.py run examples/pentest.ax
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Tuple


ARTIFACTS_DIR = Path("artifacts")
DEBUG = bool(os.environ.get("AXION_DEBUG"))


@dataclass
class Step:
    type: str
    data: Dict[str, object]


@dataclass
class ScenarioData:
    steps: List[Step] = field(default_factory=list)
    imports: List[str] = field(default_factory=list)


def parse_value(raw: str) -> str:
    raw = raw.strip()
    if not raw:
        return ""
    if raw[0] == raw[-1] and raw[0] in {'"', "'"}:
        return raw[1:-1]
    return raw


def load_scenario(path: Path) -> ScenarioData:
    visited: set[Path] = set()
    return _load_recursive(path.resolve(), visited)


def _load_recursive(path: Path, visited: set[Path]) -> ScenarioData:
    if path in visited:
        return ScenarioData()
    visited.add(path)

    content = path.read_text(encoding="utf-8")
    lines = content.splitlines()
    idx = 0
    scenario = ScenarioData()

    while idx < len(lines):
        raw_line = lines[idx]
        stripped = raw_line.strip()

        if DEBUG:
            print(f"[parse] {path}:{idx + 1}: {stripped!r}")

        if not stripped or stripped.startswith("//"):
            idx += 1
            continue

        if stripped.startswith("#"):
            idx += 1
            continue

        if stripped.startswith("import "):
            import_path = parse_value(stripped[len("import ") :])
            module_path = (path.parent / import_path).resolve()
            module = _load_recursive(module_path, visited)
            scenario.imports.append(str(module_path))
            scenario.imports.extend(module.imports)
            scenario.steps.extend(module.steps)
            idx += 1
            continue

        if stripped.startswith("let "):
            name, value = _parse_let(stripped)
            scenario.steps.append(Step("variable", {"name": name, "value": value}))
            idx += 1
            continue

        if stripped.startswith("group "):
            block, remainder, next_idx = _collect_block(lines, idx)
            name = _parse_group_header(lines[idx])
            props = _parse_key_values(block)
            scenario.steps.append(
                Step("group", {"name": name, "properties": props})
            )
            idx = next_idx
            continue

        if stripped.startswith("scan "):
            block, remainder, next_idx = _collect_block(lines, idx)
            scan_name, tool = _parse_scan_header(lines[idx])
            params = _parse_key_values(block)
            output = _parse_output_label(remainder, default=f"scan_{scan_name}")
            scenario.steps.append(
                Step(
                    "scan",
                    {"name": scan_name, "tool": tool, "params": params, "output": output},
                )
            )
            idx = next_idx
            continue

        if stripped.startswith("script "):
            block, remainder, next_idx = _collect_block(lines, idx)
            script_name = _parse_script_header(lines[idx])
            params = _parse_key_values(block)
            output = _parse_output_label(remainder, default=f"script_{script_name}")
            scenario.steps.append(
                Step(
                    "script",
                    {"name": script_name, "params": params, "output": output},
                )
            )
            idx = next_idx
            continue

        if stripped.startswith("report "):
            block, remainder, next_idx = _collect_block(lines, idx)
            report_name = _parse_report_header(lines[idx])
            includes = _parse_report_includes(block)
            scenario.steps.append(
                Step("report", {"name": report_name, "includes": includes})
            )
            if remainder.strip():
                raise ValueError(f"Unexpected trailing contents in report block: {remainder}")
            idx = next_idx
            continue

        raise ValueError(f"Unsupported syntax in line: {raw_line}")

    return scenario


def _parse_let(line: str) -> Tuple[str, str]:
    rest = line[len("let ") :].strip()
    if "=" not in rest:
        raise ValueError(f"Invalid let syntax: {line}")
    name, value = rest.split("=", 1)
    name = name.strip()
    if not name:
        raise ValueError(f"Invalid variable name in: {line}")
    return name, parse_value(value)


def _parse_group_header(line: str) -> str:
    prefix, _ = line.split("{", 1)
    tokens = prefix.split()
    if len(tokens) < 2:
        raise ValueError(f"Invalid group header: {line}")
    return tokens[1]


def _parse_scan_header(line: str) -> Tuple[str, str]:
    head, _ = line.split("{", 1)
    tokens = head.split()
    if len(tokens) >= 4 and tokens[2] == "using":
        return tokens[1], tokens[3]
    if len(tokens) >= 3:
        return tokens[1], tokens[2]
    raise ValueError(f"Invalid scan header: {line}")


def _parse_script_header(line: str) -> str:
    head, _ = line.split("{", 1)
    tokens = head.split()
    if len(tokens) < 2:
        raise ValueError(f"Invalid script header: {line}")
    return tokens[1]


def _parse_report_header(line: str) -> str:
    head, _ = line.split("{", 1)
    tokens = head.split()
    if len(tokens) < 2:
        raise ValueError(f"Invalid report header: {line}")
    return tokens[1]


def _parse_output_label(remainder: str, default: str) -> str:
    trimmed = remainder.strip()
    if not trimmed:
        return default
    if not trimmed.startswith("->"):
        raise ValueError(f"Unexpected trailing content: {remainder}")
    label = trimmed[2:].strip()
    if not label:
        return default
    return label


def _parse_key_values(block_lines: List[str]) -> Dict[str, str]:
    result: Dict[str, str] = {}
    for raw in block_lines:
        stripped = raw.strip()
        if not stripped or stripped.startswith("#") or stripped.startswith("//"):
            continue
        parts = stripped.split(None, 1)
        if len(parts) != 2:
            raise ValueError(f"Invalid key/value line: {raw}")
        key = parts[0]
        value = parse_value(parts[1])
        result[key] = value
    return result


def _parse_report_includes(block_lines: List[str]) -> List[str]:
    includes: List[str] = []
    for raw in block_lines:
        stripped = raw.strip()
        if not stripped or stripped.startswith("#") or stripped.startswith("//"):
            continue
        if not stripped.startswith("include "):
            raise ValueError(f"Unsupported directive in report: {raw}")
        target = stripped[len("include ") :].strip()
        includes.append(parse_value(target))
    return includes


def _collect_block(lines: List[str], start_index: int) -> Tuple[List[str], str, int]:
    header = lines[start_index]
    if "{" not in header:
        raise ValueError(f"Missing '{{' in block header: {header}")
    before, after = header.split("{", 1)
    body: List[str] = []
    if after.strip():
        body.append(after)

    idx = start_index + 1
    remainder = ""
    while idx < len(lines):
        current = lines[idx]
        closing_pos = _find_closing_brace(current)
        if closing_pos is None:
            body.append(current)
            idx += 1
            continue

        before_close = current[:closing_pos]
        after_close = current[closing_pos + 1 :]
        if before_close.strip():
            body.append(before_close)
        remainder = after_close.strip()
        idx += 1
        if DEBUG:
            print(
                f"[collect] start={start_index} end={idx} remainder={remainder!r} body={body}"
            )
        break
    else:
        raise ValueError(f"Block not closed for header: {lines[start_index]}")

    return body, remainder, idx


def _find_closing_brace(line: str) -> int | None:
    idx = 0
    length = len(line)
    while idx < length:
        char = line[idx]
        if char == "$" and idx + 1 < length and line[idx + 1] == "{":
            end = line.find("}", idx + 2)
            if end == -1:
                return None
            idx = end + 1
            continue
        if char == "}":
            return idx
        idx += 1
    return None


def substitute(value: str, variables: Dict[str, str]) -> str:
    result = []
    idx = 0
    while idx < len(value):
        start = value.find("${", idx)
        if start == -1:
            result.append(value[idx:])
            break
        result.append(value[idx:start])
        end = value.find("}", start + 2)
        if end == -1:
            raise ValueError(f"Unterminated variable placeholder in '{value}'")
        name = value[start + 2 : end].strip()
        if not name:
            raise ValueError("Empty variable placeholder")
        if name not in variables:
            raise ValueError(f"Undefined variable '{name}' in '{value}'")
        result.append(variables[name])
        idx = end + 1
    return "".join(result)


def sanitize_label(label: str) -> str:
    return "".join(c if c.isalnum() or c in "-_." else "_" for c in label)


def ensure_artifacts_dir() -> None:
    ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)


def build_summary(scenario: ScenarioData) -> Dict[str, object]:
    imports = sorted(set(scenario.imports))
    variables: List[Dict[str, str]] = []
    groups: List[Dict[str, object]] = []
    scans: List[Dict[str, object]] = []
    scripts: List[Dict[str, object]] = []
    reports: List[Dict[str, object]] = []

    for step in scenario.steps:
        if step.type == "variable":
            variables.append({"name": step.data["name"], "value": step.data["value"]})
        elif step.type == "group":
            groups.append({"name": step.data["name"], "properties": step.data["properties"]})
        elif step.type == "scan":
            scans.append(
                {
                    "name": step.data["name"],
                    "tool": step.data["tool"],
                    "output": step.data.get("output"),
                }
            )
        elif step.type == "script":
            scripts.append(
                {
                    "name": step.data["name"],
                    "output": step.data.get("output"),
                }
            )
        elif step.type == "report":
            reports.append(
                {
                    "name": step.data["name"],
                    "includes": step.data["includes"],
                }
            )

    return {
        "total_steps": len(scenario.steps),
        "imports": imports,
        "variables": variables,
        "asset_groups": groups,
        "scans": scans,
        "scripts": scripts,
        "reports": reports,
    }


def format_summary(summary: Dict[str, object]) -> str:
    lines = [f"Steps: {summary['total_steps']}"]
    imports = summary.get("imports", [])
    if imports:
        lines.append("Imports:")
        for item in imports:
            lines.append(f"  - {item}")
    variables = summary.get("variables", [])
    if variables:
        lines.append("Variables:")
        for var in variables:
            lines.append(f"  - {var['name']} = {var['value']}")
    groups = summary.get("asset_groups", [])
    if groups:
        lines.append("Asset groups:")
        for group in groups:
            props = ", ".join(f"{k}={v}" for k, v in group["properties"].items())
            lines.append(f"  - {group['name']} ({props})")
    scans = summary.get("scans", [])
    if scans:
        lines.append("Scans:")
        for scan in scans:
            output = scan.get("output") or "<auto>"
            lines.append(f"  - {scan['name']} via {scan['tool']} -> {output}")
    scripts = summary.get("scripts", [])
    if scripts:
        lines.append("Scripts:")
        for script in scripts:
            output = script.get("output") or "<auto>"
            lines.append(f"  - {script['name']} -> {output}")
    reports = summary.get("reports", [])
    if reports:
        lines.append("Reports:")
        for report in reports:
            includes = ", ".join(report["includes"])
            lines.append(f"  - {report['name']} (includes: {includes})")
    return "\n".join(lines)


def execute_scenario(
    scenario: ScenarioData, json_mode: bool
) -> Tuple[Dict[str, object], List[Dict[str, object]], List[Dict[str, object]]]:
    ensure_artifacts_dir()
    variables: Dict[str, str] = {}
    artifacts: Dict[str, Dict[str, object]] = {}
    execution_steps: List[Dict[str, object]] = []

    for step in scenario.steps:
        if step.type == "variable":
            value = substitute(step.data["value"], variables)
            variables[step.data["name"]] = value
            execution_steps.append(
                {
                    "name": step.data["name"],
                    "type": "variable",
                    "status": "completed",
                    "message": f"{step.data['name']} = {value}",
                }
            )
        elif step.type == "group":
            try:
                props = {
                    key: substitute(value, variables)
                    for key, value in step.data["properties"].items()
                }
                artifact_name = f"asset_group:{step.data['name']}"
                data = {"name": step.data["name"], "properties": props}
                path = _write_artifact(artifact_name, data)
                artifacts[artifact_name] = {
                    "name": artifact_name,
                    "kind": "asset_group",
                    "path": str(path) if path else None,
                    "data": data,
                }
                execution_steps.append(
                    {
                        "name": step.data["name"],
                        "type": "asset_group",
                        "status": "completed",
                        "message": f"stored asset group ({artifact_name})",
                    }
                )
            except Exception as exc:
                execution_steps.append(
                    {
                        "name": step.data["name"],
                        "type": "asset_group",
                        "status": "failed",
                        "message": str(exc),
                    }
                )
        elif step.type == "scan":
            result = _execute_scan(step.data, variables)
            artifacts[result["name"]] = result["artifact"]
            execution_steps.append(result["execution"])
        elif step.type == "script":
            result = _execute_script(step.data, variables)
            artifacts[result["name"]] = result["artifact"]
            execution_steps.append(result["execution"])
        elif step.type == "report":
            result = _execute_report(step.data, variables, artifacts, json_mode)
            artifacts[result["name"]] = result["artifact"]
            execution_steps.append(result["execution"])
        else:
            execution_steps.append(
                {
                    "name": step.data.get("name", "<unknown>"),
                    "type": step.type,
                    "status": "skipped",
                    "message": "unsupported step type",
                }
            )

    artifacts_list = list(artifacts.values())
    summary = build_summary(scenario)
    return summary, execution_steps, artifacts_list


def _write_artifact(label: str, data: Dict[str, object]) -> Path | None:
    try:
        safe_label = sanitize_label(label)
        path = ARTIFACTS_DIR / f"{safe_label}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
        return path
    except Exception:
        return None


def _execute_scan(step_data: Dict[str, object], variables: Dict[str, str]) -> Dict[str, object]:
    params = {
        key: substitute(value, variables)
        for key, value in step_data["params"].items()
    }
    tool = step_data["tool"]
    output_label = step_data.get("output") or f"scan_{step_data['name']}"

    command = [tool]
    if "flags" in params:
        command.extend(shlex.split(params["flags"]))
    if "args" in params:
        command.extend(shlex.split(params["args"]))
    if params.get("target"):
        command.append(params["target"])

    cwd = params.get("cwd") or None
    started = time.time()

    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            check=False,
        )
        duration_ms = int((time.time() - started) * 1000)
        artifact_data = {
            "tool": tool,
            "params": params,
            "invocation": command,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
            "exit_code": completed.returncode,
            "duration_ms": duration_ms,
        }
        path = _write_artifact(output_label, artifact_data)
        status = "completed" if completed.returncode == 0 else "failed"
        message = f"{tool} exit {completed.returncode}, artifact: {path or '<memory>'}"
    except FileNotFoundError:
        status = "failed"
        artifact_data = {
            "tool": tool,
            "params": params,
            "invocation": command,
            "stdout": "",
            "stderr": f"tool '{tool}' not found",
            "exit_code": None,
            "duration_ms": 0,
        }
        path = _write_artifact(output_label, artifact_data)
        message = f"tool '{tool}' not found"

    return {
        "name": output_label,
        "artifact": {
            "name": output_label,
            "kind": "scan",
            "path": str(path) if path else None,
            "data": artifact_data,
        },
        "execution": {
            "name": step_data["name"],
            "type": "scan",
            "status": status,
            "message": message,
        },
    }


def _execute_script(step_data: Dict[str, object], variables: Dict[str, str]) -> Dict[str, object]:
    params = {
        key: substitute(value, variables)
        for key, value in step_data["params"].items()
    }
    output_label = step_data.get("output") or f"script_{step_data['name']}"

    run_value = params.get("run")
    if not run_value:
        return {
            "name": output_label,
            "artifact": {
                "name": output_label,
                "kind": "script",
                "path": None,
                "data": {"error": "missing run parameter"},
            },
            "execution": {
                "name": step_data["name"],
                "type": "script",
                "status": "failed",
                "message": "missing required parameter: run",
            },
        }

    command = shlex.split(run_value)
    if "args" in params:
        command.extend(shlex.split(params["args"]))
    cwd = params.get("cwd") or None

    started = time.time()
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            check=False,
        )
        duration_ms = int((time.time() - started) * 1000)
        artifact_data = {
            "command": command,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
            "exit_code": completed.returncode,
            "duration_ms": duration_ms,
        }
        path = _write_artifact(output_label, artifact_data)
        status = "completed" if completed.returncode == 0 else "failed"
        message = f"exit {completed.returncode}, artifact: {path or '<memory>'}"
    except FileNotFoundError:
        artifact_data = {
            "command": command,
            "stdout": "",
            "stderr": f"command '{command[0]}' not found",
            "exit_code": None,
            "duration_ms": 0,
        }
        path = _write_artifact(output_label, artifact_data)
        status = "failed"
        message = artifact_data["stderr"]

    return {
        "name": output_label,
        "artifact": {
            "name": output_label,
            "kind": "script",
            "path": str(path) if path else None,
            "data": artifact_data,
        },
        "execution": {
            "name": step_data["name"],
            "type": "script",
            "status": status,
            "message": message,
        },
    }


def _execute_report(
    step_data: Dict[str, object],
    variables: Dict[str, str],
    artifacts: Dict[str, Dict[str, object]],
    json_mode: bool,
) -> Dict[str, object]:
    includes = [substitute(include, variables) for include in step_data["includes"]]
    included_data: Dict[str, object] = {}

    for name in includes:
        if name not in artifacts:
            raise ValueError(f"Missing artifact '{name}' for report '{step_data['name']}'")
        included_data[name] = artifacts[name]["data"]

    report_data = {
        "name": step_data["name"],
        "includes": included_data,
    }
    label = f"report:{step_data['name']}"
    path = _write_artifact(label, report_data)

    if step_data["name"] == "stdout":
        rendered = json.dumps(report_data, indent=2, ensure_ascii=False)
        print(rendered)

    execution = {
        "name": step_data["name"],
        "type": "report",
        "status": "completed",
        "message": f"report stored at {path or '<memory>'}",
    }

    return {
        "name": label,
        "artifact": {
            "name": label,
            "kind": "report",
            "path": str(path) if path else None,
            "data": report_data,
        },
        "execution": execution,
    }


def plan_command(args: argparse.Namespace) -> None:
    scenario = load_scenario(Path(args.input))
    summary = build_summary(scenario)
    if args.json:
        print(json.dumps(summary, indent=2, ensure_ascii=False))
    else:
        print(format_summary(summary))


def run_command(args: argparse.Namespace) -> None:
    scenario = load_scenario(Path(args.input))
    summary, execution, artifacts = execute_scenario(scenario, args.json)
    if args.json:
        payload = {
            "summary": summary,
            "execution": execution,
            "artifacts": artifacts,
        }
        print(json.dumps(payload, indent=2, ensure_ascii=False))
        return

    print(format_summary(summary))
    print("\nExecution:")
    for step in execution:
        status = step["status"]
        print(f"  - {step['type']} {step['name']}: {status} ({step['message']})")
    failures = [step for step in execution if step["status"] == "failed"]
    if failures:
        print("\n[warn] some steps failed")
    if artifacts:
        print("\nArtifacts:")
        for artifact in artifacts:
            location = artifact.get("path") or "<memory>"
            print(f"  - {artifact['name']} ({artifact['kind']}) -> {location}")


def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Axion DSL Python runner")
    subparsers = parser.add_subparsers(dest="command", required=True)

    plan = subparsers.add_parser("plan", help="Parse a scenario and print summary")
    plan.add_argument("input", help="Path to the scenario file")
    plan.add_argument("--json", action="store_true", help="Output JSON")
    plan.set_defaults(func=plan_command)

    run = subparsers.add_parser("run", help="Execute a scenario")
    run.add_argument("input", help="Path to the scenario file")
    run.add_argument("--json", action="store_true", help="Output JSON")
    run.set_defaults(func=run_command)

    return parser


def main(argv: List[str] | None = None) -> None:
    if argv is None:
        argv = sys.argv[1:]

    if argv and not argv[0].startswith("-"):
        possible = Path(argv[0])
        if possible.suffix == ".ax" and possible.exists():
            argv = ["run"] + argv

    parser = build_arg_parser()
    args = parser.parse_args(argv)
    try:
        args.func(args)
    except Exception as exc:  # pragma: no cover - user-facing
        print(f"[error] {exc}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":  # pragma: no cover
    main()
