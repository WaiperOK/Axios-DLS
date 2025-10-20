# Axios DSL Manual

This manual provides the formal description of the Axios domain-specific language together with its execution environment, artifact model, and extension architecture. It targets language designers, security engineers, and toolsmiths who require a precise understanding of how Axios scenarios are authored, resolved, and executed.

## Structure

1. [Preface](preface.md) — historical context and relationship to adjacent research.
2. [Design Philosophy](philosophy.md) — guiding principles that inform the language surface and runtime behaviour.
3. [Language Fundamentals](fundamentals.md) — core constructs, scenario layout, and execution workflow.
4. [Lexical and Syntactic Grammar](syntax.md) — tokens, whitespace rules, and the canonical grammar.
5. [Operational Semantics](semantics.md) — step evaluation, variable substitution, error handling, and determinism.
6. [Runtime Architecture](runtime.md) — executor responsibilities, artifact life cycle, and concurrency model.
7. [Toolchain Integration](tooling.md) — CLI usage, module packaging, external tool adapters, and federation patterns.
8. [Artifacts and Reporting](artifacts.md) — persistent artifact formats, reporting DSL, and tabular rendering.
9. [Security and Reliability](security.md) — threat model, sandboxing expectations, and observability requirements.
10. [Integration Scenarios](integration.md) — reproducible workflows for assessments, purple team drills, and CI/CD.
11. [Glossary](glossary.md) — canonical definitions for domain vocabulary.
12. [Appendix A: Grammar Reference](appendix-grammar.md).
13. [Appendix B: Artifact Schemas](appendix-artifacts.md).

When reading sequentially, the manual progresses from conceptual framing to exhaustive specification. Each chapter is cross-referenced so that readers can navigate directly to sections relevant to implementation questions or operational deployment.
