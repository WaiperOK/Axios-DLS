# Secrets Management Proposal

## Goals

- Provide first-class handling for sensitive values (credentials, API tokens, SSH keys).
- Prevent accidental leakage of secrets through logs, artifacts, or diagnostic output.
- Support multiple secret sources (.env files, external vaults, CLI overrides).
- Maintain compatibility with existing scenarios while offering opt-in enhancements.

## Use Cases

1. Scenario author references credentials for authenticated scans (e.g., authenticated HTTP scans).
2. Ops team executes scenarios in CI and injects secrets via environment variables without editing source files.
3. Integration with HashiCorp Vault or other secret stores to fetch credentials at runtime.

## Proposed DSL Additions

### Secret Blocks

```axion
secret db_creds from env {
  username = "DB_USER"
  password = "DB_PASS"
}

secret api_key from file ".secrets/api.key"

secret vault_token from vault {
  path = "kv/data/axion"
  field = "token"
}
```

- `secret <name> from env { key = "ENV_VAR" }`: maps environment variables into runtime secret store.
- `secret <name> from file "path"`: loads a file content.
- `secret <name> from vault { path = "..." field = "..." }`: fetches from external provider (extensible).

Secrets become available via `${secret:name}` expressions (distinct from `${var}`) and are resolved lazily.

## Runtime Handling

- Extend executor with a SecretStore that holds decrypted values in memory only.
- Secrets never persisted to artifact JSON or stdout; masking applied to execution logs.
- CLI overrides (--secret name=value) to inject ephemeral secrets during execution.

## Masking Strategy

- Any log or diagnostic string that includes ${secret:...} is redacted (***).
- When writing artifacts, references are replaced with placeholders; consumers retrieve secret separately.
- Provide audit log (debug mode) that indicates secret usage without revealing value (e.g., secret api_key consumed by scan web_auth).

## External Providers

- Start with built-in providers: `env`, `file`.
- Vault integration behind feature flag: require configuration (`VAULT_ADDR`, `VAULT_TOKEN`).
- Open provider trait so third-party SDKs can register new sources.

## CLI / SDK Impact

- `axion plan` warns if secret definitions are missing required sources or referenced but undefined.
- `axion schema` to expose secret providers metadata (so IDE can offer UI for secret wiring).
- SDK: expose `SecretDescriptor { name, provider, parameters }` via `axion_core` for UI consumption.

## Incremental Implementation Plan

1. Parser/AST: add SecretStep with provider metadata.
2. Runtime: introduce SecretStore, env/file providers, masking in logging.
3. CLI overrides & plan diagnostics for secrets.
4. Optional: Vault provider integration.
5. Documentation: update security guide, quickstart, roadmap.
