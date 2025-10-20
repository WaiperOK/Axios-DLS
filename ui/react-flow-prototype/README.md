# Axion React Flow Prototype

This experimental UI lets you sketch Axion DSL scenarios as a small graph and
export the result back to a text file.

## Getting started

```bash
cd ui/react-flow-prototype
npm install
npm run dev
```

Open the printed URL (defaults to `http://localhost:5173`) and edit nodes on the
canvas. The preview panel shows the DSL snippet in real time and includes a copy
button for convenience.

## Current capabilities

- Asset → Scan → Report happy path.
- Inline editing of common parameters (CIDR, Nmap flags, includes, etc.).
- Automatic fallback values when connections imply relationships.
- Export to the Axion DSL layout used by the CLI examples.

This is only a sketch; contributions and refinements are welcome.
