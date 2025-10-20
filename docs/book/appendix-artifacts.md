# Appendix B: Artifact Schemas

The following JSON Schema fragments formalise the structure of artifacts emitted by the reference runtime. They can be used to validate artifacts in downstream systems or to generate strongly typed bindings.

## StoredArtifact

```json
{
  "type": "object",
  "required": ["name", "kind", "data"],
  "properties": {
    "name": {"type": "string"},
    "kind": {"type": "string", "enum": ["AssetGroup", "Scan", "Script", "Report"]},
    "path": {"type": ["string", "null"]},
    "data": {"type": "object"}
  },
  "additionalProperties": false
}
```

## AssetGroupArtifact

```json
{
  "type": "object",
  "required": ["name", "properties"],
  "properties": {
    "name": {"type": "string"},
    "properties": {
      "type": "object",
      "patternProperties": {
        "^[A-Za-z0-9_.-]+$": {"type": "string"}
      },
      "additionalProperties": false
    }
  }
}
```

## ScanArtifacts

```json
{
  "type": "object",
  "required": ["tool", "target", "assets", "findings", "raw_xml"],
  "properties": {
    "tool": {"type": "string"},
    "target": {"type": "string"},
    "assets": {
      "type": "array",
      "items": {"$ref": "#/$defs/Asset"}
    },
    "findings": {
      "type": "array",
      "items": {"$ref": "#/$defs/Finding"}
    },
    "raw_xml": {"type": "string"}
  },
  "$defs": {
    "Asset": {
      "type": "object",
      "required": ["id", "addresses"],
      "properties": {
        "id": {"type": "string"},
        "addresses": {
          "type": "array",
          "items": {"type": "string"}
        },
        "hostnames": {
          "type": "array",
          "items": {"type": "string"},
          "default": []
        },
        "labels": {
          "type": "object",
          "patternProperties": {
            "^[A-Za-z0-9_.-]+$": {"type": "string"}
          },
          "additionalProperties": false,
          "default": {}
        }
      }
    },
    "Finding": {
      "type": "object",
      "required": [
        "id",
        "asset_id",
        "port",
        "protocol",
        "state",
        "title",
        "description",
        "severity"
      ],
      "properties": {
        "id": {"type": "string"},
        "asset_id": {"type": "string"},
        "port": {"type": "integer", "minimum": 0, "maximum": 65535},
        "protocol": {"type": "string"},
        "state": {"type": "string"},
        "service": {"type": ["string", "null"]},
        "title": {"type": "string"},
        "description": {"type": "string"},
        "severity": {"type": "string"},
        "evidence": {
          "type": "object",
          "additionalProperties": true,
          "default": {}
        }
      }
    }
  }
}
```

## ScriptArtifact

```json
{
  "type": "object",
  "required": ["name", "command", "stdout", "stderr", "started_at", "duration_ms"],
  "properties": {
    "name": {"type": "string"},
    "command": {
      "type": "array",
      "items": {"type": "string"}
    },
    "stdout": {"type": "string"},
    "stderr": {"type": "string"},
    "exit_code": {"type": ["integer", "null"]},
    "started_at": {"type": "string", "format": "date-time"},
    "duration_ms": {"type": "number", "minimum": 0}
  }
}
```

## ReportArtifact

```json
{
  "type": "object",
  "required": ["target", "generated_at", "includes", "tables"],
  "properties": {
    "target": {"type": "string"},
    "generated_at": {"type": "string", "format": "date-time"},
    "includes": {
      "type": "object",
      "additionalProperties": true
    },
    "tables": {
      "type": "object",
      "additionalProperties": {"$ref": "#/$defs/Table"}
    }
  },
  "$defs": {
    "Table": {
      "type": "object",
      "required": ["columns", "rows"],
      "properties": {
        "columns": {
          "type": "array",
          "items": {"type": "string"}
        },
        "rows": {
          "type": "array",
          "items": {
            "type": "object",
            "additionalProperties": true
          }
        }
      }
    }
  }
}
```

These schemas may be imported into OpenAPI specifications, validation pipelines, or code generators to ensure consistent handling of Axios artifacts across heterogeneous environments.
