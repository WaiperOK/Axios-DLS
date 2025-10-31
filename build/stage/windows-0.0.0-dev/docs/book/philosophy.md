# Design Philosophy

The Axios language is guided by five interrelated principles that balance expressiveness, auditability, and operational safety.

## Declarative Orchestration

Axios scenarios describe *what* should be discovered, inspected, or reported rather than prescribing low-level control flow. Each step captures intent and key parameters, leaving the executor to derive command-line invocations, data capture, and artifact management. This separation permits higher-level reasoning about the scenario while ensuring consistent execution.

## Deterministic Reproducibility

Two executions of the same scenario, under equivalent environmental conditions, must produce equivalent artifacts. Determinism is achieved by explicitly naming external tools, enumerating parameters, and capturing command outputs within the Axios artifact store. Implicit environmental dependencies are treated as defects to be surfaced through validation warnings.

## Composability Through Imports

Scenarios often share discovery patterns, asset definitions, and reporting pipelines. Axios supports modular design via `import` statements that inline referenced files after cyclic dependency elimination. Imports are resolved prior to execution, yielding a flattened execution plan that is easier to audit and reason about.

## Introspection-First Runtime

Every evaluator decision, tool invocation, intermediate result, and generated report is stored as structured data. Artifacts are designed for downstream analytics, enabling teams to build dashboards, feed knowledge graphs, or replay findings without rerunning intrusive scans. The runtime favours transparency over silent heuristics.

## Safe Extensibility

Axios treats external tools as untrusted programs. The executor exposes a deliberately narrow integration surface: scans, scripts, and reports operate within explicit parameter sets, variable interpolation is controlled, and artifact emission is sandboxed. Future extensions—such as remote execution backends or parallel schedulers—must preserve these boundaries to keep scenarios auditable and safe to share.
