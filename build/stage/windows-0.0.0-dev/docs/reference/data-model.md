# CLI and Runtime Data Model

This document captures the structures emitted by the Axios CLI and runtime. All payloads are JSON-serialisable and stable across minor releases.

## Execution Report

```
{
  "steps": [
    {
      "name": "discovery",
      "kind": "Scan",
      "status": "Completed",
      "message": "nmap executed. exit: 0. artifact: artifacts/discovery.json"
    }
  ]
}
```

| Field    | Description                                                      |
|----------|------------------------------------------------------------------|
| `name`   | Directive identifier.                                            |
| `kind`   | Enumeration: `AssetGroup`, `Scan`, `Variable`, `Script`, `Report`. |
| `status` | Enumeration: `Completed`, `Failed`, `Skipped`, `NotImplemented`. |
| `message`| Optional human-readable details.                                 |

## Execution Outcome

When invoking `axion-cli run --json`, the CLI returns:

```
{
  "summary": { ... ExecutionReport ... },
  "execution": { ... ExecutionReport ... },
  "artifacts": [
    {
      "name": "discovery",
      "kind": "Scan",
      "path": "artifacts/discovery.json",
      "data": { ... }
    }
  ]
}
```

`summary` mirrors the planning view, while `execution` reflects actual results. `artifacts` embeds serialised artifacts (see below).

## Artifact Schemas

### StoredArtifact

| Field | Description |
|-------|-------------|
| `name` | Artifact label (`asset_group:perimeter`, `report:stdout`, etc.). |
| `kind` | One of `AssetGroup`, `Scan`, `Script`, `Report`. |
| `path` | Filesystem location if persisted, otherwise `null`. |
| `data` | Artifact payload (type-specific schema). |

### AssetGroupArtifact

```
{
  "name": "perimeter",
  "properties": {
    "scope": "external",
    "cidr": "10.0.0.0/24"
  }
}
```

### ScanArtifacts (Nmap Specialisation)

```
{
  "tool": "nmap",
  "target": "10.0.0.0/24",
  "assets": [
    {
      "id": "10.0.0.5",
      "addresses": ["10.0.0.5"],
      "hostnames": [],
      "labels": {}
    }
  ],
  "findings": [
    {
      "id": "10.0.0.5:22/tcp",
      "asset_id": "10.0.0.5",
      "port": 22,
      "protocol": "tcp",
      "state": "open",
      "service": "ssh",
      "title": "Open SSH service",
      "description": "Service fingerprint derived from Nmap probe.",
      "severity": "informational",
      "evidence": {}
    }
  ],
  "raw_xml": "<nmaprun>...</nmaprun>"
}
```

### ScriptArtifact

```
{
  "name": "fingerprint",
  "command": ["python3", "tools/fingerprint.py", "--target", "10.0.0.0/24"],
  "stdout": "...",
  "stderr": "",
  "exit_code": 0,
  "started_at": "2025-10-20T18:30:12Z",
  "duration_ms": 742
}
```

### ReportArtifact

```
{
  "target": "stdout",
  "generated_at": "2025-10-20T18:31:02Z",
  "includes": {
    "discovery": { ... ScanArtifacts ... },
    "fingerprint": { ... ScriptArtifact ... }
  },
  "tables": {
    "discovery": {
      "columns": ["asset_id", "port", "protocol", "service", "state", "severity", "description"],
      "rows": [ ... ]
    }
  }
}
```

For precise validation, leverage the JSON Schemas in `docs/reference/schemas/` and the appendix in the manual (`docs/book/appendix-artifacts.md`).
