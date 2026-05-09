# EPIC Contract: vb-qi37 — Master-Doc Completion Gap Plan

## EPIC Overview

**Bead ID:** vb-qi37
**Title:** (EPIC) release: Master-doc completion gap plan
**Status:** OPEN — Phase 1 (Contract)
**Phase ordering anchor:** MASTER.md Sections 35, 43; Phase 36 DoD gate

This EPIC coordinates 7 child beads targeting the gap between current Round 2 implementation state and the Phase 36 hardening / final DoD. Children are grouped into three bands: **foundation** (storage/journal), **evidence** (primitive contracts, tests), and **gate** (runtime hardening, verification). All children must reach State 8 (Landed) before this EPIC closes.

---

## Child Bead Tracking Summary

| ID | Title | Crate(s) | State | Band |
|----|-------|----------|-------|------|
| vb-fb52 | storage: Atomic journal and index write batches | vb_storage | 1-Contract | **Foundation** |
| vb-2yb8 | runtime/storage: Per-primitive durability proof matrix | vb_runtime, vb_storage | 1-Contract | **Evidence** |
| vb-78f9 | verifier/runtime: Action contract schema validation | vb_validate, vb_runtime | 1-Contract | **Evidence** |
| vb-6azo | quality: Behavioral property tests for workflow engine invariants | vb_core, vb_runtime | 1-Contract | **Evidence** |
| vb-7gs9 | runtime: Shard scheduler bounded ownership evidence | vb_runtime | 1-Contract | **Gate** |
| vb-2bok | verifier/storage: Durability gate for accepted artifacts | vb_validate, vb_storage | 1-Contract | **Gate** |
| vb-99n6 | runtime: Timer wheel driven resume and cancellation hardening | vb_runtime | 1-Contract | **Gate** |

---

## Dependency Ordering Between Children

### Band 1 — Foundation (must close first)

**vb-fb52** (atomic journal/index batches) is a hard prerequisite for:
- vb-2yb8 — durability proofs require atomic journal writes to emit deterministic proof sequences
- vb-2bok — accepted-artifact durability gate requires atomic batchpersistence of `RunAccepted` headers

**Constraint:** vb-fb52 must reach at least State 4 (Evidence) before vb-2yb8 and vb-2bok enter State 5 (Evidence Gate).

### Band 2 — Evidence (require vb-fb52 foundation; parallel among themselves)

**vb-2yb8**, **vb-78f9**, **vb-6azo** are independent of each other and may proceed in parallel once vb-fb52 is proven.

- vb-2yb8 produces the per-primitive durability proof matrix (MASTER.md Section 40 evidence chain requirement)
- vb-78f9 validates the Action ABI contract schema at compile/run time
- vb-6azo provides proptest-based behavioral property tests for engine invariants

**Constraint:** All three Band-2 beads must close before Band-3 gates open.

### Band 3 — Gate (require Band 2 evidence; independent among themselves)

**vb-7gs9**, **vb-2bok**, **vb-99n6** form the final hardening layer.

- vb-7gs9 proves shard scheduler bounded ownership (no cross-shard state leakage, one-owner-per-run)
- vb-2bok gates accepted artifacts on durability evidence (complements vb-2yb8 proof matrix)
- vb-99n6 hardens timer-wheel resume and cancellation paths (wait/ask/retry/timer determinism)

**Constraint:** vb-7gs9 and vb-99n6 may close in either order; vb-2bok requires vb-2yb8 evidence.

---

## Phase Ordering (Foundation Before Evidence Gates)

```
[vb-fb52] ──────────────────────────────────────────────────────────► [vb-2yb8]
  Foundation                                                           Evidence
  Atomic write batches                                                 Durability proof matrix
                                                                      │
                                                                      ▼
                                                                    [vb-2bok]
                                                               Durability gate
                                                               (requires vb-2yb8)
                                                                       
[vb-78f9] ─────────────────────────────────────────────────────────► (same close window)
  Action contract schema validation

[vb-6azo] ─────────────────────────────────────────────────────────► (same close window)
  Behavioral property tests

[vb-7gs9] ─────────────────────────────────────────────────────────► (closes independently)
  Shard scheduler ownership evidence

[vb-99n6] ─────────────────────────────────────────────────────────► (closes independently)
  Timer wheel hardening
```

**Parallelism:** Band 2 beads (vb-2yb8, vb-78f9, vb-6azo) run in parallel after vb-fb52 foundation is laid. Band 3 beads (vb-7gs9, vb-2bok, vb-99n6) run in parallel after Band 2 closes, except vb-2bok depends on vb-2yb8.

---

## Integration Points

| Point | Involved Children | Contract |
|-------|------------------|----------|
| Journal record envelope | vb-fb52 → vb-2yb8 | `encode_record`/`decode_record` (MASTER.md Section 18) must be stable and atomic; vb-2yb8 durability proofs enumerate every `record_kind_u16` event sequence |
| Action ABI | vb-78f9 → vb-2yb8 | Action `Idempotency` enum and `ActionTicket` shape (MASTER.md Section 19) must be schema-validated; vb-2yb8 proves replay safety for each `Idempotency` variant |
| Shard ownership | vb-7gs9 → vb-2bok | `ShardCommand::Submit` (MASTER.md Section 20) is the only admission path; vb-7gs9 proves no run is admitted without shard-bound command; vb-2bok gates artifact acceptance on this |
| Timer wheel | vb-99n6 → vb-7gs9 | `TimerFired` command (MASTER.md Section 20) must correctly route to owning shard; vb-7gs9 covers run-stays-on-one-shard invariant; vb-99n6 covers timer ordering determinism |
| Accepted artifact | vb-2bok ← vb-2yb8, vb-fb52 | `RunAccepted` event (record_kind=10) must be atomically persisted before acknowledgement; vb-2yb8 proof matrix documents this boundary per primitive |
| Property tests | vb-6azo → all | Proptest invariants (MASTER.md Section 38) for state machine transitions, taint safety, replay determinism, ordering invariants must be satisfied before final DoD |

---

## Master-Doc Gap Alignment

This EPIC closes gaps mapped in MASTER.md Section 35 Round 2 DoD table and the black-hat findings in Section 42:

| MASTER.md Gap | Child Bead(s) |
|---------------|---------------|
| Recovery evidence chain (Phase 40) | vb-2yb8 (durability proofs), vb-2bok (artifact gate) |
| Runtime full live-frame recovery hydration (Phase 33 gap) | vb-7gs9 (shard ownership), vb-99n6 (timer resume) |
| Action ABI semantic validation (Phase 18 gap) | vb-78f9 (action schema validation) |
| Property/mutation/invariant test coverage (Phase 36) | vb-6azo (behavioral property tests) |
| Atomic journal/index batches (Phase 16 gap) | vb-fb52 (atomic write batches) |

---

## EPIC Acceptance Criteria

1. All 7 child beads reach State 8 (Landed) with test/benchmark evidence.
2. vb-fb52 closes before vb-2yb8 and vb-2bok enter State 5.
3. Band 2 (vb-2yb8, vb-78f9, vb-6azo) closes before Band 3 (vb-7gs9, vb-2bok, vb-99n6) enters State 5.
4. vb-2bok requires vb-2yb8 evidence present in the bead database.
5. `bd dolt push` syncs this EPIC and all children to remote.
6. MASTER.md Round 2 status table entries are updated to reflect closed evidence for all gaps above.

---

## Out of Scope for This EPIC

- Full IR/generated parity equivalence (future phase)
- CLI completeness (separate bead group)
- Makepad UI beads (separate bead group)
- Fuzz target expansion (covered by separate beads)
- PGO/maxperf build engineering (Phase 35, separate bead)
