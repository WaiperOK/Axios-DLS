# Security and Reliability

Scenarios orchestrate potentially destructive tooling. A disciplined security posture is therefore mandatory for both authors and operators. This chapter outlines the baseline threat model, operational safety guidelines, and observability expectations for Axios deployments.

## Threat Model

1. **Untrusted tooling**: External binaries invoked by `scan` or `script` directives may be compromised. Contain execution within hardened hosts, containers, or sandboxes.
2. **Scenario provenance**: Imported modules can inject arbitrary directives. Source scenarios from version-controlled repositories with code review and signed commits.
3. **Artifact leakage**: Generated artifacts often contain sensitive infrastructure metadata. Enforce access controls on the `artifacts/` directory and purge data after analysis.
4. **Variable injection**: Interpolated variables can alter command lines. Restrict variable sources to vetted configuration files or secrets managers.

## Execution Safety

- Run scenarios against test environments before targeting production infrastructure.
- Employ rate limiting and throttling at the tool level (`nmap` flags, custom script arguments) to prevent overwhelming services.
- Remain compliant with legal and contractual obligations. Axios does not embed safeguards against scanning assets without authorisation.
- Adopt a principle of least privilege when granting network access or credentials to scan hosts.

## Reliability Practices

- **Idempotent scenarios**: Ensure repeated runs do not lead to divergent states. For example, scripts should avoid destructive changes unless explicitly intended.
- **Error surfacing**: Treat any `failed` step as a prompt for triage. Missing artifacts cascade into report failures; design reports to highlight upstream errors.
- **State isolation**: Execute scenarios in dedicated working directories to avoid mixing artifacts between engagements.

## Observability

- Capture CLI `--json` output as part of engagement logs. Pair with system-level telemetry (process start/stop, network connections) for audit trails.
- Monitor artifact directory changes using file integrity monitoring if scenarios run on shared hosts.
- Integrate with SIEM systems by forwarding report artifacts or translating findings into formats such as SARIF or STIX (future extensions).

## Supply Chain Integrity

- Vendoring modules: Pin imports to specific commit hashes when referencing external repositories.
- Rust dependencies: Use `cargo audit` to track vulnerabilities in the executor and CLI.
- External binaries: Validate signatures or checksums of third-party tools prior to execution.

## Incident Response

- In the event of a suspected compromise, halt scenario execution, archive artifacts for forensic analysis, and rebuild execution hosts from trusted images.
- Rotate any credentials or access tokens referenced within variables or scripts.
- Document findings and update modules to prevent recurrence.

Security assurance is not a one-time undertaking. Treat this guidance as the baseline, and layer environment-specific controls (network segmentation, privileged access management, logging) to match your organisational risk tolerance.
