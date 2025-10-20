# Artifacts and Reporting

Artifacts transform transient execution output into durable knowledge. They capture the structured state of each step and form the basis for reporting, analytics, and post-engagement review.

## Artifact Lifecycle

1. **Creation**: Step handlers populate a strongly typed artifact (`AssetGroupArtifact`, `ScanArtifact`, etc.).
2. **Storage**: The artifact is serialized into JSON and stored within the executor's in-memory map. When persistence succeeds, the artifact is also written to `artifacts/<label>.json`.
3. **Consumption**: Reports and external tooling read artifacts either directly from disk or through the execution outcome emitted by the CLI.
4. **Retention**: Operators determine retention policies. Sensitive data—such as raw scan output—should be scrubbed or encrypted when stored outside controlled environments.

## Artifact Types

### AssetGroupArtifact

```
{
  "name": "perimeter",
  "properties": {
    "scope": "external",
    "location": "/opt/targets/external.csv"
  }
}
```

- Captures metadata about assets or scopes.
- Enables documentation of discovery context that scans can reference.

### ScanArtifacts

```
{
  "tool": "nmap",
  "target": "198.51.100.0/24",
  "assets": [
    {
      "id": "198.51.100.23",
      "addresses": ["198.51.100.23"],
      "hostnames": ["www.example.test"],
      "labels": {"source": "nmap"}
    }
  ],
  "findings": [
    {
      "id": "198.51.100.23:443/tcp",
      "asset_id": "198.51.100.23",
      "port": 443,
      "protocol": "tcp",
      "state": "open",
      "service": "https",
      "title": "Open TLS service",
      "description": "Service fingerprint derived from Nmap probe.",
      "severity": "informational",
      "evidence": {
        "banner": "Apache httpd 2.4.58"
      }
    }
  ],
  "raw_xml": "<nmaprun>...</nmaprun>"
}
```

- Provides both structured findings and the original XML for reproducibility.
- Findings carry stable identifiers (`asset_id`, `port`, `protocol`) to facilitate deduplication.

### ScriptArtifact

```
{
  "name": "fingerprint_web",
  "command": ["python3", "tools/fingerprint.py", "--url", "https://198.51.100.23"],
  "stdout": "...",
  "stderr": "",
  "exit_code": 0,
  "started_at": "2025-10-20T16:40:12Z",
  "duration_ms": 732
}
```

- Mirrors generic process execution metadata.
- Enables reproducibility checks by recording the exact invocation vector.

### ReportArtifact

```
{
  "target": "stdout",
  "generated_at": "2025-10-20T16:45:01Z",
  "includes": {
    "findings_discovery": { ... scan artifact ... }
  },
  "tables": {
    "findings_discovery": {
      "columns": ["asset_id", "port", "protocol", "service", "state", "severity", "description"],
      "rows": [ ... ]
    }
  }
}
```

- Consolidates referenced artifacts and derived tables.
- Additional report backends may extend the schema with channel-specific metadata.

## Artifact Names and Paths

- The artifact `name` is determined by the directive:
  - Asset groups: `asset_group:<identifier>`.
  - Scans and scripts: either `<directive_name>` or the alias provided via `-> alias`.
  - Reports: `report:<target>`.
- `path` contains the filesystem location when persistence succeeds; otherwise it is `null`.
- Consumers should rely on the artifact `name` rather than the path to maintain portability.

## Reporting Best Practices

1. **Explicit Includes**: List every artifact consumed by a report to ensure dependency visibility.
2. **Tailored Targets**: When exporting beyond stdout, choose report targets such as `report sarif { ... }` (future extension) to align with consumer expectations.
3. **Artifact Normalisation**: Normalise severity, classification, or evidence fields within scripts so that reports need minimal post-processing.

Artifacts are the authoritative record of scenario execution. Maintaining their fidelity is central to Axios' goal of transforming ad hoc assessments into auditable, repeatable processes.
