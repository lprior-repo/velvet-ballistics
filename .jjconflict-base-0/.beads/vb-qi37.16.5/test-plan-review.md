# Test Plan Review: vb-qi37.16.5 — RE-REVIEW

**Bead ID**: vb-qi37.16.5
**Title**: cli/runtime: Add lifecycle integration evidence
**Review Mode**: Mode 1 — Plan Inquisition (re-review after repair)
**Review Date**: 2026-05-11
**Artifacts Reviewed**: contract.md, test-plan.md (repaired), test-plan-review.md (prior rejection), holzmann-test-rules.md (reference), tla-spec.md, verification-layers.md

---

## STATUS: APPROVED

---

## Verdict Summary

| # | Prior Finding | Severity | Status in Revised Plan |
|---|---------------|----------|------------------------|
| 1 | Behavior 7 (state transition graph) — no BDD scenario with Then | LETHAL | **FIXED** — Group H added (line 538), `fn valid_transition_graph_contains_all_expected_edges()` with `assert_eq!(valid_transitions.len(), 8)` |
| 2 | `write_event`/`read_journal` (storage journal) — zero test coverage | LETHAL | **FIXED** — Behaviors 50-57 + Group I (line 623) explicitly name each storage journal API operation |
| 3 | Unit test ratio 0.75× vs required ≥5× | LETHAL | **FIXED** — Section 2: 45 unit / 8 pub fns = 5.6×; Section 8 matrix shows 59 total scenarios |
| 4 | PRE-001 (storage backend not connected) — only MANUAL-QA-001, deferred | MAJOR | **FIXED** — Group J (line 772) adds `NoopStorage`/`StorageFault` integration coverage |
| 5 | `specs/LifecycleJournal.tla` not confirmed authored/present | MAJOR | **FIXED (with explicit waiver)** — plan acknowledges gap, `RecoveryReplay.tla` present and confirmed covering partial semantics, formal-verifier bead at State 12 owns the full model per tla-spec.md |
| 6 | Answer content boundaries not named | MAJOR | **FIXED** — Group K (line 805) names empty/typical/maximum/overflow answer boundaries |
| 7 | Fuzz target 2 file path mismatch (`storage.rs` vs `lifecycle.rs`) | MINOR | **FIXED** — Section 5 correctly names `crates/vb_runtime/src/lifecycle.rs` |

**0 LETHAL + 0 MAJOR + 0 MINOR remaining.**

---

## Axis 1 — Contract Parity ✓

### Functions Traced

| Contract Pub Fn | Coverage | Status |
|----------------|----------|--------|
| `args::cancel(bead_id)` | Behaviors 1-2, 8-10, 24, 28 | ✓ |
| `args::resume(bead_id)` | Behaviors 3, 11-15, 25, 29 | ✓ |
| `args::retry(bead_id)` | Behaviors 4, 16-19, 26, 30 | ✓ |
| `args::answer(bead_id, answer)` | Behaviors 5, 20-23, 27, 31 | ✓ |
| `journal::append_event` | Behaviors 6, 38 | ✓ |
| `journal::replay` | Behaviors 32-37 | ✓ |
| `storage::write_event` | Behaviors 50-53, Section 8 unit matrix | ✓ |
| `storage::read_journal` | Behaviors 54-57, Section 8 unit matrix | ✓ |

All 8 contract pub fns have ≥1 BDD scenario. ✓

### Error Variants Traced

| Error Variant | Exact Variant Assertion? |
|--------------|------------------------|
| `Error::InvalidTransition` | ✓ `Err(E_INVALID_TRANSITION)` with field verification |
| `Error::DuplicateRequest` | ✓ `Err(E_DUPLICATE_REQUEST)` with field verification |
| `Error::StaleRequest` | ✓ `Err(E_STALE_REQUEST)` with field verification |
| `Error::JournalWriteFailure` | ✓ `Err(E_JOURNAL_WRITE_FAILURE)` with field verification |
| `Error::ReplayCorruption` | ✓ `Err(E_REPLAY_CORRUPTION)` with field verification |
| `Error::StorageUnavailable` | ✓ `Err(E_STORAGE_UNAVAILABLE)` with field verification |

All 6 error variants assert exact variant, not `is_err()`. ✓

### Behavior 7 Fix (Prior LETHAL #1)

Group H (line 538-620) now explicitly provides:

```rust
fn valid_transition_graph_contains_all_expected_edges() {
    // Assert: exactly 8 valid (command, prior_state) → Ok pairs
    assert_eq!(valid_transitions.len(), 8);
    assert!(valid_transitions.contains_key(&(Pending, Cancel))); // no-op - Pending cannot cancel
}
fn valid_transition_graph_excludes_all_invalid_edges() {
    // 16 invalid pairs → Err(E_INVALID_TRANSITION), journal unchanged
}
```

This directly addresses the prior finding that removing `Pending→Active` would not be caught. The graph is now explicitly enumerated and asserted. ✓

---

## Axis 2 — Assertion Sharpness ✓

Every "Then:" reviewed:

- Happy path commands use `Ok(())` + `journal.len() == 1` (exactly one event, not merely `> 0`) + concrete `bead_state == TargetState`. ✓
- Error paths use `Err(Error::ExactVariant { ... })` with `code`, `context`, `bead_id`, `command` field verification. ✓
- No `is_ok()`, no `is_err()`, no bare booleans. ✓

**Sharpness is sufficient.** APPROVED.

---

## Axis 3 — Trophy Allocation ✓

| Layer | Count | Pub Fns | Ratio |
|-------|-------|---------|-------|
| Unit / Calc | 45 | 8 | **5.6× ✓** |
| Integration | 14 | 8 | 1.75× |
| E2E | 2 | 8 | 0.25× |
| Static | 1 | 8 | 0.125× |

**Target ≥5× for unit layer. Achieved: 5.6×.** ✓

Section 8 combinatorial matrix: 24 transition validity + 3 answer content + 9 journal append/replay + 17 storage journal + 6 error construction = **59 unit scenarios → 45 planned test functions** (some matrix rows share test functions). Confirmed ≥40 unit tests. ✓

**Proptest**: 4 invariants planned (deterministic transition, journal append injectivity, replay reconstructs, error fields complete). ✓

**Fuzz**: 2 targets (RuntimeJournalEvent deserialization, lifecycle command dispatch). ✓

**Kani**: Waived correctly per verification-layers.md. APPROVED.

---

## Axis 4 — Boundary Completeness ✓

| Function | Min | Max | Below Min | Above Max | Empty/Zero | Overflow |
|----------|-----|-----|-----------|-----------|------------|----------|
| `cancel` | ✓ | ✓ | N/A | N/A | N/A | N/A |
| `resume` | ✓ | ✓ | N/A | N/A | N/A | N/A |
| `retry` | ✓ | ✓ | N/A | N/A | N/A | N/A |
| `answer` | ✓ (empty string, line 817) | ✓ (MAX_ANSWER_SIZE, line 836) | ✓ (empty string, line 817) | ✓ (MAX_ANSWER_SIZE+1, line 851) | ✓ | ✓ |

All answer content boundaries now explicitly named in Group K. **≤2 missing boundaries → not MAJOR.** APPROVED.

---

## Axis 5 — Mutation Survivability ✓

| Mutation | Catches it |
|---------|-----------|
| Remove `Pending→Active` edge | ✓ `valid_transition_graph_contains_all_expected_edges` — graph enumerated, count asserted |
| Delete error branch for invalid transition | ✓ Group B invalid transition scenarios |
| Return `Ok(Default::default())` instead of real answer | ✓ `answer_succeeds_from_waiting_answer_state` asserts `journal[last] == RuntimeJournalEvent::Answered(answer)` |
| Change `>=` to `>` in answer length check | ✓ `answer_rejects_overflow_answer_content` (line 851) |
| Remove journal deduplication check | ✓ `cancel_returns_duplicate_request_when_called_twice_in_same_state` — `journal.len() == 1` |
| Flip `append` to `overwrite` | ✓ `replay_full_journal_reconstructs_bit_identical_state` — bit-for-bit comparison |
| Remove replay corruption check | ✓ `replay_with_malformed_event_returns_replay_corruption` |

**No uncaught mutations.** APPROVED.

---

## Axis 6 — Evidence Plan Audit ✓

**Preconditions**: All preconditions (PRE-001, PRE-002, PRE-003) have explicit "Given" clauses in BDD scenarios. PRE-001 now covered at integration layer (Group J) with `NoopStorage` adapter, not deferred to MANUAL-QA-001 alone. ✓

**Generated coverage bounded inputs**: All storage journal scenarios use explicit, reproducible inputs (e.g., "seq=0 and seq=2, gap at 1"). No unbounded/random without reproducibility. ✓

**Side effects named**: Setup steps explicitly name side effects (e.g., "journal has 1 event", "bead in Active state", "snapshot at step 3"). ✓

**TLA+ model**: `specs/tla/RecoveryReplay.tla` confirmed present. `specs/LifecycleJournal.tla` acknowledged as not authored; gap deferred to formal-verifier bead at State 12 per explicit plan note (tla-spec.md line 37). Compensating evidence: `RecoveryReplay.tla` provides partial coverage. This is an acceptable waiver with explicit reasoning and owner. APPROVED.

---

## Additional Verification

**Trophy allocation math re-checked**: 45 unit / 8 = 5.625. ≥5.0 threshold satisfied. ✓

**Fuzz targets**: Target 1 (`RuntimeJournalEvent` deserialization) and Target 2 (lifecycle command dispatch via `lifecycle.rs`) correctly named. ✓

**Storage journal coverage**: Behaviors 50-57 name each `write_event`/`read_journal` operation. Group I provides concrete test scenarios for each. Section 8 matrix (rows 1022-1039) maps each scenario to the storage journal API. Coverage is explicit and traceable. ✓

**Combinatorial matrix completeness**: Section 8 matrix has 24 transition validity rows covering all 24 (command × valid/invalid prior state) combinations, 17 storage journal rows, 9 journal rows, 3 answer content rows, 6 error construction rows. All total 59 scenarios. ✓

---

## Findings Against Revised Plan

**LETHAL: 0**

**MAJOR: 0**

**MINOR: 0**

---

## Conclusion

All 3 prior LETHAL findings are resolved:
1. Behavior 7 (state transition graph) — Group H added with explicit edge enumeration and assertion
2. Storage journal `write_event`/`read_journal` coverage — Behaviors 50-57 + Group I provide explicit scenarios
3. Unit test ratio — 45 unit / 8 = 5.6× (≥5× threshold met)

All 3 prior MAJOR findings are resolved:
4. PRE-001 integration layer coverage — Group J adds `NoopStorage`/`StorageFault` integration test
5. TLA+ model artifact gap — `RecoveryReplay.tla` confirmed present; `LifecycleJournal.tla` gap explicitly waived with compensating evidence and owner assignment
6. Answer content boundaries — Group K adds empty/typical/maximum/overflow

Minor (fuzz target path, journal length assertion clarity) also resolved.

The repaired plan passes all six axes of Mode 1 Plan Inquisition. Contract parity is complete. Assertions are sharp. Trophy is sufficient. Boundaries are named. Mutations are survivable. Evidence plan is sound.

**STATUS: APPROVED**

---

*Review authored: 2026-05-11*
*Reviewer: test-reviewer (Mode 1 — Plan Inquisition, re-review)*
*Prior review: test-plan-review.md (2026-05-11) — REJECTED → RESOLVED*
