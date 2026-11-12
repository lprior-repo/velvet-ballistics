# Proof Strategy — vb-rpch
## State: 4 (Proof Planning)

---

## Scope

**Bead**: vb-rpch — BDD: Durability and recovery acceptance scenarios
**Verifiers**: verus + tla-plus + kani + bdd_scenario
**Domain**: Journal replay, Fjall persistence, RecoveryFrameSeed, ActionReplayTracker, DigestCheck
**Primary crates**: vb_storage, vb_runtime, vb_core

---

## TLA+ Strategy

### Current State
`specs/tla/RecoveryReplay.tla` EXISTS with 92 lines covering idempotency theorems:
- `NoDuplicateNonIdempotent` — no two non-idempotent actions scheduled for same (run,step,action,attempt)
- `ReplaySafe` — completed action+step is never followed by a rescheduled same action+step in same attempt

`specs/tla/RecoveryReplay.cfg` EXISTS but is INCOMPLETE:
- Defines constants: RunId={1,2}, StepId={1,2}, ActionId={1}, Attempt={0,1}
- Missing: SPECIFICATION, INIT, INVARIANT declarations, PROPERTY declarations

### Gap Analysis
The existing TLA spec is NARROWER than the 6 TLA-owned clauses in the contract:
- **TLA-001 (ReplaySeqOrder)**: NOT modeled — seq ordering and step monotonicity not in spec
- **TLA-002 (TailCausalAfterSnapshot)**: NOT modeled — snapshot-plus-tail ordering not in spec
- **TLA-003 (OnlyIncompleteRuns)**: NOT modeled — DiscoverIncomplete action not in spec
- **TLA-004 (NoResolvedReExecution)**: PARTIALLY modeled — covered by `ReplaySafe` theorem
- **TLA-005 (RecoveryErrorExhaustive)**: NOT modeled — error reachability not modeled
- **TLA-006 (DigestVerificationOrder)**: NOT modeled — digest check ordering not in spec

### Required TLA+ Work
The proof-writer must either:
1. EXTEND `specs/tla/RecoveryReplay.tla` with full variables, actions, and invariants for TLA-001/002/003/005/006; or
2. SPLIT into two specs: `RecoveryReplayIdempotency.tla` (existing narrow spec) and `RecoveryReplayFull.tla` (full pipeline)

Recommendation: SPLIT to preserve the existing theorems and add a new spec for the full pipeline.

### TLA+ Evidence Command
```
cd /home/lewis/src/velvet-ballistics
tlc -config specs/tla/RecoveryReplay.cfg specs/tla/RecoveryReplay.tla
```
Expected: TLC reports 0 errors, theorems `Spec => []NoDuplicateNonIdempotent` and `Spec => []ReplaySafe` PROVED.

**DISCOVERY_BLOCKED for TLA-001/002/003/005/006**: The cfg is incomplete and the spec does not cover these clauses. `recovery_replay_full.cfg` and `recovery_replay_full.tla` do not exist yet. Proof-writer must create them.

---

## Verus Strategy

### Current State
NO Verus annotations exist in `crates/vb_storage/src/recovery/types.rs`, `replay/core.rs`, or `hydrate.rs`.
All 7 Verus obligations in `proof-obligations.jsonl` are PLANNED but not written.

### Required Verus Work (7 proof obligations)
| Obligation | File | spec/proof fn | Status |
|---|---|---|---|
| VERUS-INV-002 | types.rs | union_invariant, union_commutative, union_associative, union_idempotent, union_no_contradiction | MISSING |
| VERUS-INV-004 | types.rs | tracker_resolved_invariant, tracker_mark_completed_preserves_monotonicity, tracker_mark_failed_preserves_monotonicity | MISSING |
| VERUS-INV-005 | types.rs | hierarchy_invariant, digest_check_strict_hierarchy | MISSING |
| VERUS-PRE-001 | hydrate.rs | precondition_invariants, preconditions_enforced | MISSING |
| VERUS-PRE-002 | hydrate.rs | precondition_invariants, preconditions_enforced | MISSING |
| VERUS-POST-009 | replay/core.rs | attempt_filter_invariant, replay_events_respects_attempt_filter | MISSING |
| VERUS-INV-003 | replay/summary.rs | seed_dimension_invariants, seed_construction_preserves_dimensions | MISSING |

### Verus Evidence Command
```
cd /home/lewis/src/velvet-ballistics
verus crates/vb_storage/src/recovery/types.rs crates/vb_storage/src/recovery/replay/core.rs crates/vb_storage/src/recovery/hydrate.rs
```
Expected: Verus verified 0 errors on all proof obligations.

**DISCOVERY_BLOCKED**: Verus must be run AFTER proof-writer adds spec/proof fns to the source files.

---

## Kani Strategy

### Current State
Kani harnesses do not exist yet. The obligations reference `hydrate_run_frame_precond_kani`, `hydrate_run_frame_from_events_precond_kani`, and `replay_events_kani` but these are planned by the proof-writer.

### Required Kani Work (3 proof obligations)
| Obligation | Harness | Status |
|---|---|---|
| KANI-PRE-001 | hydrate_run_frame_precond_kani | PLANNED |
| KANI-PRE-002 | hydrate_run_frame_from_events_precond_kani | PLANNED |
| KANI-POST-009 | replay_events_kani | PLANNED |

### Kani Evidence Command
```
cargo kani --harness hydrate_run_frame_precond_kani --no-unwind
cargo kani --harness hydrate_run_frame_from_events_precond_kani --no-unwind
cargo kani --harness replay_events_kani --no-unwind
```
Expected: Kani reports no panic for preconditions on bounded JournalEvent sequences.

---

## BDD Strategy

### Current State
`crates/vb_storage/tests/recovery_bdd_tests.rs` EXISTS with 1919 lines covering B-001 through B-017.
All BDD tests are in `vb_storage` crate under `tests/recovery_bdd_tests.rs`.

### BDD Evidence Command
```
cargo test -p vb_storage --test recovery_bdd_tests -- B-001a --exact --nocapture
# ... (each BDD test individually)
```
All 22 BDD obligations (B-001a through B-017 plus special: B-HYDRATE, B-REJECT, B-CORRUPT) plus 1 durability gate test (BDD-STRICT-DUR) must pass.

---

## Waivers

| Clause | Reason | Owner | Expiry |
|---|---|---|---|
| GAP-3 ActionAbiMismatch | Lookup function not implemented; GAP-3 tracked in vb-ty9 | vb-ty9 owner | vb-ty9 |
| GAP-3 PolicyDigestMismatch | Lookup function not implemented; GAP-3 tracked in vb-ty9 | vb-ty9 owner | vb-ty9 |
| TerminalStateMismatch | No expected-terminal parameter in public APIs; DEFERRED_GLOBAL B-017 | vb-oewy owner | TBD |

---

## Obligation Execution Order

1. **verus**: Proof-writer adds spec/proof fns to source files, then run Verus to verify
2. **kani**: Proof-writer writes harnesses, then run Kani to verify panic-freedom
3. **tla-plus**: Proof-writer completes RecoveryReplay.cfg and extends spec for TLA-001/002/003/005/006, then run TLC
4. **bdd**: Run all BDD tests via `moon run :bdd`
5. **moon run :perf**: Run durability gate test

---

## Risk Summary

| Risk | Verifier | Status |
|---|---|---|
| UnsupportedRecoveryState::union algebraic properties | verus | MISSING |
| ActionReplayTracker::is_resolved monotonicity | verus | MISSING |
| DigestCheck hierarchy strictness | verus | MISSING |
| hydrate_run_frame preconditions (no panic) | kani | MISSING |
| hydrate_run_frame_from_events preconditions (no panic) | kani | MISSING |
| replay_events attempt filtering (no panic) | kani | MISSING |
| Journal replay seq ordering | tla-plus | INCOMPLETE |
| Snapshot-plus-tail causal consistency | tla-plus | NOT MODELLED |
| Incomplete run discovery | tla-plus | NOT MODELLED |
| Non-idempotent action blocking | tla-plus | PARTIAL |
| RecoveryError reachability | tla-plus | NOT MODELLED |
| Digest verification ordering | tla-plus | NOT MODELLED |
| BDD scenario coverage (22 tests) | bdd | UNKNOWN |
| Durability strict profile | moon :perf | UNKNOWN |
