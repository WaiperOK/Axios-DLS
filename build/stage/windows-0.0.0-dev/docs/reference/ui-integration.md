# UI Integration Notes

## Loading Tool Schemas

- Run `axion schema --format json > ui/react-flow-prototype/src/assets/tool-schemas.json` during build or dev.
- Watch the `version` field; regenerate when the CLI updates.
- For live edit, consider a small local API that proxies `axion schema` results.

## Secrets (Future)

- Once `secret` blocks land, the UI should provide editors for secret providers (env/file/vault).
- Tool schemas will expose provider metadata so forms can show required fields.

