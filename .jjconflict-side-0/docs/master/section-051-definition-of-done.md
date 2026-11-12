---
section: 51
title: "Definition of Done"
parent: velvet-ballistics-MASTER.md
---

## 51. Definition of Done

The SDK-first backend milestone is complete when all conditions hold:

1. Workflows are authored only through the Rust SDK DSL.
2. YAML workflow authoring is removed or legacy-migration-only.
3. `velvet_workflow!` rejects arbitrary Rust behavior inside workflow bodies.
4. `VelvetInput`/`VelvetOutput` derive bounded schemas and digests.
5. Action manifests expose side effects, retry safety, idempotency scope, capabilities, secrets, schemas, timeouts, and output bounds.
6. Action executors are separate from manifests.
7. Compiler verifies type, control flow, boundedness, effect, idempotency, capability, secret, taint, durability, and result gates.
8. Accepted artifacts bind source digest, IR digest, action ABI digest, policy digest, resource budget digest, and certificate digest.
9. Runtime accepts accepted artifacts only by default.
10. Capability requirements and operator grants are distinct types and distinct records.
11. Policy profile names never stand in for policy digests.
12. Idempotency key expressions are AST values and reject secrets/time/random/env/attempt by default.
13. External side effects require durable outbox/history evidence before dispatch.
14. Completions require durable tickets.
15. Duplicate completion handling is deterministic.
16. Non-idempotent replay is blocked or reconciled, never accidentally re-executed.
17. Runtime executes numeric IR only.
18. Runtime has no YAML/JSON/HTTP core dependency.
19. Hot runtime zones have no unsafe/unwrap/expect/panic/unchecked indexing/slicing/casts/arithmetic/unbounded allocation.
20. All list/event/report APIs are bounded.
21. Testkit uses production compiler/runtime/storage/replay paths.
22. Required compile-fail tests pass.
23. Required fuzz/property/replay/crash tests pass.
24. All performance claims have benchmark evidence.
25. Evidence bundles exist for every closed phase/bead.
26. `cargo velvet verify -> simulate -> artifact` and `velvet-ballistics submit -> incident -> replay` work as the canonical demo.

---

