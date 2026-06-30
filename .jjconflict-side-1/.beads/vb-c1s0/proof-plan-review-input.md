# Proof Plan Review Input: vb-c1s0

**Bead:** vb-c1s0 — BDD Orchestration Runtime Acceptance Scenarios  
**State:** 4 (Proof Planning)  
**Reviewer:** contract-verification-reviewer  
**Artifacts:** 3 (proof-strategy.md, proof-plan-review-input.md, proof-obligations.planned.jsonl)

---

## Obligation Summary

| Layer | Count | Critical |
|-------|-------|----------|
| TLA+ | 6 | 3 temporal + 2 routing + 1 lifecycle |
| Verus | 7 | INV-002 through INV-006 + PRE-001 + PRE-004 |
| Kani | 5 | tick_all, Shard::tick, timer_insert, queue, frame |
| Miri | 1 | action_queue Send+Sync + UB |
| Loom | 2 | tick_all concurrency, queue concurrency |
| Proptest | 1 | primitives |
| Integration | 4 | BDD + CLI + catalog |
| Gauntlet | 2 | proof gate + all gate |
| **Total** | **28** | |

---

## Risk-to-Verifier Map

| Risk | Primary | Secondary |
|------|---------|-----------|
| Temporal routing (INV-001) | TLA-WF-001 | KANI-TICK-001 |
| FIFO tick (INV-007) | TLA-WF-002 | KANI-SHARD-001, LOOM-SHARD-001 |
| Terminal state (POST-002) | TLA-WF-003 | INTEGRATION-BDD-001 |
| Timer authority (INV-003, POST-004) | TLA-WF-004 | VERUS-INV-003, VERUS-PRE-004 |
| Action routing (POST-003) | TLA-WF-005 | KANI-QUEUE-001 |
| Shard shutdown (POST-005) | TLA-WF-006 | KANI-TICK-001 |
| Timer gen (INV-002) | VERUS-INV-002 | TLA-WF-004 |
| Queue FIFO (INV-004) | VERUS-INV-004 | MIRI-QUEUE-001 |
| Queue capacity (INV-005) | VERUS-INV-005 | KANI-QUEUE-001 |
| Budget exhaustion (INV-006) | VERUS-INV-006 | KANI-FRAME-001 |
| Runtime construction (PRE-001) | VERUS-PRE-001 | — |
| Timer fired precond (PRE-004) | VERUS-PRE-004 | — |

---

## Waiver Candidates

1. **TLA-WF-006** — shares ShardProcessing model with TLA-WF-002. Waive if TLA-WF-002 passes with ShutdownCorrectness invariant added.
2. **LOOM-SHARD-001** — KANI-TICK-001 provides bounded panic-freedom; Loom is additional. Waive if loom blocked_tooling.
3. **PROPTEST-PRIM-001** — Low risk; BDD integration covers primitives. Waive if primitives have no broad input space.

---

## Missing Artifacts at State 4

| Artifact | Status | Blocker |
|----------|--------|---------|
| TLA+ specs (5 .tla + 5 .cfg) | MISSING | proof-writer must create |
| Verus spec/proof fns | MISSING | proof-writer must add annotations |
| Kani harnesses | MISSING | proof-writer must create |
| Loom models | MISSING | proof-writer must create |
| Integration tests (recovery_bdd, cli) | UNKNOWN | Check in velvet-ballistics source |
| Miri test binary | UNKNOWN | Requires test file to exist |

---

## Verification Layer Assignments

| Clause | Owner | Mode |
|--------|-------|------|
| INV-001 | TLA+ | verify-proof |
| INV-002 | Verus | verify-proof |
| INV-003 | Verus | verify-proof |
| INV-004 | Verus | verify-proof |
| INV-005 | Verus | verify-proof |
| INV-006 | Verus | verify-proof |
| INV-007 | TLA+ | verify-proof |
| PRE-001 | Verus | verify-proof |
| PRE-004 | Verus | verify-proof |
| POST-002 | TLA+ | verify-proof |
| POST-003 | TLA+ | verify-proof |
| POST-004 | TLA+ | verify-proof |
| POST-005 | TLA+ | verify-proof |
| POST-006 | Verus | verify-proof |

---

## Open Questions Requiring Resolution

| ID | Question | Impact |
|----|----------|--------|
| OQ-001 | New vs. existing BDD scenarios? | INTEGRATION-CATALOG-001 scope |
| OQ-002 | Compound workflow scheduling scope? | TLA-WF-005 model bounds |
| OQ-003 | TLA+ specs exist in source checkout? | TLA+ obligation creation path |
| OQ-004 | Verus annotations in source? | Verus obligation creation path |
| OQ-005 | Loom models exist? | Loom obligation creation path |

---

## Recommendation

**APPROVE with conditions:** All 28 obligations are properly mapped to contract clauses and risk tags. Waiver candidates are reasonable. proof-writer must confirm artifact existence before state 5. Resolve OQ-001 through OQ-005 before finalizing TLA+ and Verus scopes.
