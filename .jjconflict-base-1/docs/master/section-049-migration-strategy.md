---
section: 49
title: "Migration Strategy"
parent: velvet-ballistics-MASTER.md
---

## 49. Migration Strategy

Migration from YAML-first to SDK-first proceeds in phases:

1. Freeze YAML workflow authoring; no new features.
2. Introduce `vb_sdk`, `vb_sdk_macros`, `vb_action`, `vb_policy`, and `vb_artifact`.
3. Implement `velvet_workflow!` for the minimal deterministic/action/wait/ask/retry surface.
4. Implement derives for `VelvetInput`, `VelvetOutput`, and `VelvetData`.
5. Implement action manifest/executor split.
6. Implement idempotency key AST and verifier.
7. Implement policy digesting and capability grants split.
8. Make `cargo velvet verify` the hero command.
9. Emit accepted artifacts from SDK workflow definitions.
10. Change runtime admission to require accepted artifacts by default.
11. Add migration tool from legacy YAML to SDK source.
12. Remove YAML crates from active workspace.
13. Delete or quarantine old YAML tests and docs.
14. Refresh all examples around SDK source.
15. Update definition of done.

No release may claim SDK-first completion while YAML remains an active production authoring path.

---

