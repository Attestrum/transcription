# Diagrams — the design contract

Architecture here is **diagram-first**: for any new module, IPC command
surface, on-disk schema, or multi-step flow, a Mermaid diagram lands in this
tree before (or with) the code that implements it. Mermaid is the only
authoring format — GitHub renders it natively. PNGs rendered by
`tools/render-diagrams.sh` are derived, local-only review artifacts
(gitignored, never committed).

Every diagram file carries frontmatter:

```yaml
---
title: "<short imperative description>"
models: "<the code or spec this diagram describes>"
source_of_truth: code      # or: diagram
last_verified: <commit SHA short> <YYYY-MM-DD>
diagram_type: flowchart    # or stateDiagram-v2, sequenceDiagram, classDiagram, erDiagram
---
```

- `source_of_truth: diagram` — the diagram is the contract the code must
  implement (planning phase). It flips to `code` once the implementation
  lands.
- `source_of_truth: code` — code is authoritative; the diagram is a derived
  view. If a change to the named code alters described behavior, update the
  diagram in the same commit and bump `last_verified`.

Pick the type that fits: `flowchart` for pipelines and decision trees,
`stateDiagram-v2` for lifecycles, `sequenceDiagram` for multi-actor flows,
`classDiagram` for stable public APIs, `erDiagram` for on-disk schemas.
