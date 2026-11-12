---
section: 7
title: "Nightly Governance"
parent: velvet-ballistics-MASTER.md
---

## 7. Nightly Governance


Nightly is required to target peak performance and strict lint behavior. It is not permission to use unstable APIs casually.

Nightly update contract:

1. Nightly version changes require a dedicated bead.
2. The bead must record current nightly, target nightly, motivation, changed compiler behavior, and rollback plan.
3. Full CI, Miri, fuzz smoke, benchmarks, and recovery tests must pass. Generated Rust compile tests are not current-scope gates.
4. Benchmark deltas must be recorded before and after the update.
5. Any new lint allowance requires explicit documented justification.

---
