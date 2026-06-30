---
section: 53
title: "Killer Demo Contract"
parent: velvet-ballistics-MASTER.md
---

## 53. Killer Demo Contract

The first public demo must show:

```text
1. AI writes a Rust SDK workflow.
2. `cargo velvet verify` rejects unsafe retry on external write.
3. Diagnostic includes structured repair with idempotency key guidance.
4. AI patches workflow.
5. `cargo velvet verify` accepts.
6. `cargo velvet simulate` runs with mocks and emits event history.
7. `cargo velvet artifact` emits `.vbir` accepted artifact.
8. `velvet-ballistics submit` admits artifact under operator grants.
9. A simulated action timeout occurs.
10. `velvet-ballistics incident` explains whether side effect occurred and whether retry is safe.
11. `velvet-ballistics replay` reconstructs the run from durable history.
```

If an AI can answer “what failed, is it safe to retry, and what should change?” using only the structured reports, the architecture is working.

---

