# Integration Scenarios

Axios scenarios serve as building blocks for broader security programmes. This chapter illustrates representative workflows that demonstrate how the language integrates with existing processes and tooling.

## Continuous Attack Surface Assessment

1. **Inventory ingestion**: Import asset groups generated from CMDB exports or cloud inventories.
2. **Discovery scans**: Schedule nightly `nmap` scenarios with conservative timing flags.
3. **Differential analysis**: Compare successive `ScanArtifacts` to detect new hosts, ports, or services.
4. **Ticketing integration**: Convert findings above a severity threshold into issue tracker entries using a custom report backend.

## Purple Team Exercises

1. **Collaborative authoring**: Blue and red teams co-author scenarios, encoding adversary emulation steps (`script` directives) and detection validation queries.
2. **Execution windows**: Run scenarios during predefined windows to ensure monitoring coverage and stakeholder awareness.
3. **Telemetry capture**: Collect artifacts alongside SIEM alert streams. Use the structured outputs to reconcile expected and observed detections.
4. **Post-exercise reporting**: Extend reports to include scoring metrics, control gaps, and remediation instructions.

## CI/CD Pipeline Integration

1. **Pre-deployment checks**: Embed Axios scenarios in release pipelines to validate staging environments (open ports, outdated services).
2. **Policy enforcement**: Fail pipeline stages when critical findings are detected, optionally exposing summaries through `--json`.
3. **Artifact archiving**: Store scenario outputs as build artifacts for traceability and post-mortem review.

## Third-Party Assessment Coordination

1. **Scope definition**: Share canonical scenarios with external partners to communicate approved targets and tooling parameters.
2. **Execution reports**: Receive artifacts from partners, verify integrity, and replay results without granting them persistent network access.
3. **Knowledge base**: Curate long-lived modules capturing proven test cases and remediation narratives.

## Incident Simulations

1. **Trigger conditions**: Automate scenario execution when telemetry indicates high-risk patterns (e.g., newly exposed services).
2. **Targeted scripts**: Run verification scripts that capture forensic data for incident responders.
3. **Reporting**: Generate rapid-response reports tailored for leadership and technical teams.

These examples emphasise the dual nature of Axios scenarios: they encode both the procedural steps of an assessment and the operational data required to audit, replay, and extend those steps. Organisations are encouraged to adapt the templates to their governance and regulatory context.
