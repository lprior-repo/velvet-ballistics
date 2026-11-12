---
section: 2
title: "Canonical Names"
parent: velvet-ballistics-MASTER.md
---

## 2. Canonical Names

| Concept | Canonical spelling |
|---|---|
| Product | `velvet-ballistics` |
| Binary | `velvet-ballistics` |
| Cargo subcommand | `cargo velvet` |
| Rust SDK crate | `velvet_sdk` or `velvet_ballistics_sdk` |
| Runtime crate prefix | `vb_*` |
| Accepted artifact extension | `.vbir` |
| Workflow source files | `.rs` containing `velvet_workflow!` definitions |
| Policy format | canonical binary/Postcard or TOML source compiled to digest |
| Machine output | JSON for cold CLI/agent output, Postcard for binary artifacts |

YAML is removed from active workflow authoring. New active commands, examples, docs, tests, fixtures, schema files, and diagnostics must not use YAML workflow authoring. Migration-only references must be labeled `legacy_yaml`.

---

