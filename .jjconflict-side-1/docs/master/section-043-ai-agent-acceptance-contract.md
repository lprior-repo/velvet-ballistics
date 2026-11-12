---
section: 43
title: "AI Agent Acceptance Contract"
parent: velvet-ballistics-MASTER.md
---

## 43. AI Agent Acceptance Contract


Every implementation PR or handoff must report:

```text
1. Phase implemented.
2. Beads touched.
3. Files changed.
4. New public functions/types.
5. Error model.
6. Resource bounds.
7. Allocation behavior.
8. Hot-path behavior.
9. Fjall persistence behavior if touched.
10. IPC behavior if touched.
11. Tests added.
12. Benchmarks added.
13. Commands run.
14. Remaining follow-up work filed as beads.
```

Automatic rejection triggers:

```text
uses unsafe
uses unwrap/expect/panic/todo/unimplemented/dbg
unchecked indexing/slicing
unchecked arithmetic/casts
ignored Result
unbounded queue/loop/retry/fanout
YAML interpreted at runtime
JSON inserted into runtime core
HTTP inserted into runtime core
HashMap<String, Value> runtime state
one task per step
no tests for new code
speed claim without real benchmark baseline/result evidence
new velvet-ballistics spelling outside the exact allowlist
```

---
