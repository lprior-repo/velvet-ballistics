---
section: 1
title: "Non-Negotiable Product Boundaries"
parent: velvet-ballistics-MASTER.md
---

## 1. Non-Negotiable Product Boundaries

1. Runtime executes accepted artifacts only.
2. Runtime never executes SDK macro source, arbitrary Rust closures, YAML, JSON, HTTP handlers, or text commands.
3. Workflows compile to numeric IR over numeric slots, numeric steps, numeric expressions, numeric actions, bounded side tables, and resource contracts.
4. The compiler verifies behavior before artifact emission.
5. A run binds immutably to exactly one accepted artifact digest.
6. Durable history is the source of truth; in-memory frame state is a replay-derived cache.
7. External side effects leave the runtime only through durable action schedule evidence.
8. Action completions are accepted only when they match durable tickets.
9. Retry of side-effecting work requires idempotency attestation and compiler-validated key expressions.
10. Operator capability grants are distinct from artifact capability requirements.
11. Policy profile names are not trust boundaries; policy digests are.
12. Test runtimes prove nothing unless they use the production compiler, artifact, runtime, storage, and replay code paths.
13. Warnings are not verification unless the active policy promotes them to errors.
14. Explicit secret-condition rejection is not full control-flow taint proof.
15. All public claims about performance, durability, recovery, and safety require executable evidence.

---

