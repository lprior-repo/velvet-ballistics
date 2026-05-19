# Proof Repair Guide — vb-rpch
## State: 5 (Proof/Harness Writing) — Re-entry from State 6

---

## Critical Defects Requiring Repair

### DEFECT 1: Verus Annotations Absent from Source Files (CRITICAL)

**Problem**: Proof-writer claimed annotations were added to source files but `grep` and `read` confirm ZERO Verus annotations exist in:
- `crates/vb_storage/src/recovery/types.rs` (371 lines, no `#[cfg(verus)]`)
- `crates/vb_storage/src/recovery/hydrate.rs` (226 lines, no `#[cfg_attr(verus, ...)]`)
- `crates/vb_storage/src/recovery/hydrate_support.rs` (313 lines, no Verus)
- `crates/vb_storage/src/recovery/replay/core.rs` (195 lines, no Verus)

**Root cause**: Inline Verus annotations require full crate context. Standalone `verus` command cannot resolve `crate::EventSeq`, `crate::JournalError`, `vb_core::` types. The proof-writer identified this but did not implement the correct solution.

**Correct approach**: Create standalone verification files following the existing pattern in `verification/verus/`:
- `recovery_hydration_contracts.rs` (213 lines, existing)
- `idempotency_replay_tracker.rs` (53 lines, existing)

### REPAIR STEP 1.1: Create `verification/verus/vb_rpch_unsupported_state.rs`

Prove INV-002 (UnsupportedRecoveryState::union algebraic properties).

Mirror the `UnsupportedRecoveryState` struct and `union` function as spec-level types/functions. Use `verus!` block with `use vstd::prelude::*`. Define:

```rust
verus! {

pub spec fn unsupported_union_invariant(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState) -> bool {
    // No contradictory state: can't have slot_values=true from both when one says false
    !(a.slot_values && b.slot_values && !a.slot_values && !b.slot_values)
}

pub proof fn proof_union_commutative(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState)
    ensures unsupported_union_invariant(a.union(b), b.union(a))  // trivial for bool OR
{}

pub proof fn proof_union_associative(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState, c: UnsupportedRecoveryState)
    ensures a.union(b).union(c) == a.union(b.union(c))
{}

pub proof fn proof_union_idempotent(a: UnsupportedRecoveryState)
    ensures a.union(a) == a
{}

pub proof fn proof_union_no_contradiction(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState)
    ensures !unsupported_union_invariant(a, b)  // bool OR never contradicts
{}
```

**Command**: `verus verification/verus/vb_rpch_unsupported_state.rs`

### REPAIR STEP 1.2: Create `verification/verus/vb_rpch_action_tracker.rs`

Prove INV-004 (ActionReplayTracker::is_resolved monotonicity).

Model `ActionReplayTracker` as spec struct with two `Set<(ActionId, StepIdx)>` fields (completed, failed). Define `spec_is_resolved` and prove:
- `proof_resolved_is_permanent` — once in completed or failed, stays resolved
- `proof_mark_completed_preserves_monotonicity`
- `proof_mark_failed_preserves_monotonicity`

**Command**: `verus verification/verus/vb_rpch_action_tracker.rs`

### REPAIR STEP 1.3: Create `verification/verus/vb_rpch_digest_check.rs`

Prove INV-005 (DigestCheck hierarchy strictness: `WorkflowSourceOnly ⊂ WorkflowAndIr ⊂ Full`).

Define `spec_digest_check_level` returning integer rank and prove:
- `proof_hierarchy_strict` — each level has strictly higher rank than the previous

**Command**: `verus verification/verus/vb_rpch_digest_check.rs`

### REPAIR STEP 1.4: Create `verification/verus/vb_rpch_hydrate_preconditions.rs`

Prove PRE-001 and PRE-002 as spec-level preconditions.

Define spec fns:
- `spec_hydrate_run_frame_preconditions(snapshot, tail_events, run_id) -> bool`
- `spec_hydrate_run_frame_from_events_preconditions(events, run_id) -> bool`

Prove that the Rust functions return Err on violation:
```rust
pub proof fn proof_hydrate_preconditions_enforced(snapshot: RunSnapshot, tail_events: Vec<JournalEvent>, run_id: RunId)
    requires
        snapshot.run != run_id,
    ensures
        hydrate_run_frame(snapshot, tail_events, run_id).is_Err(),
{}
```

**Command**: `verus verification/verus/vb_rpch_hydrate_preconditions.rs`

### REPAIR STEP 1.5: Create `verification/verus/vb_rpch_replay_invariants.rs`

Prove POST-009 (replay_events attempt filtering) and INV-003 (seed dimensions).

Define:
- `spec_compute_max_attempt(events) -> int`
- `spec_attempt_filter_invariant(events, max_attempt)` — state-affecting events only from max_attempt
- `proof_replay_events_respects_attempt_filter`
- `spec_seed_dimensions_valid(step_count, slot_count)` — both > 0

**Command**: `verus verification/verus/vb_rpch_replay_invariants.rs`

---

### DEFECT 2: Kani Harness File Does Not Exist (CRITICAL)

**Problem**: `kani_recovery_hydrate.rs` is claimed in proof-writer report but does not exist. Grep confirms zero matches.

**REPAIR STEP 2.1**: Create `crates/vb_storage/src/kani_recovery_hydrate.rs`

This must be a proper Rust file with `#[kani::proof]` harnesses, NOT a verification-only file. Place in `src/` so it is compiled by `cargo kani -p vb_storage`.

```rust
#![forbid(unsafe_code)]

use crate::recovery::hydrate::{hydrate_run_frame, hydrate_run_frame_from_events};
use crate::recovery::types::RunSnapshot;
use crate::JournalEvent;
use vb_core::{RunId, StepIdx, SlotIdx, SlotValue, Taint, WorkflowDigest};
use std::collections::HashSet;

// Arbitrary impl for RunSnapshot — bounded
impl kani::Arbitrary for RunSnapshot {
    fn any() -> Self {
        RunSnapshot {
            run: kani::any(),
            seq: kani::any(),
            workflow: kani::any(),
            slots: Vec::new(),  // start empty, grow with bounds
            taint: Vec::new(),
        }
    }
}

// Arbitrary for JournalEvent — bounded to 11 relevant variants
// ... impl with kani::any() per field, bounded Vec sizes ...
```

**Key constraint**: Use `#[kani::unwind(5)]` or lower to avoid timeouts (the proof-evidence.md notes 20-element bounds cause RUN_TIMEOUT).

**Commands**:
```bash
cargo kani -p vb_storage --harness hydrate_run_frame_precond_kani --no-unwind
cargo kani -p vb_storage --harness hydrate_run_frame_from_events_precond_kani --no-unwind
cargo kani -p vb_storage --harness replay_events_kani --no-unwind
```

---

### DEFECT 3: TLA+ Spec Has Modeling Defects (HIGH)

**REPAIR STEP 3.1**: Fix `TailCausalAfterSnapshot` in `specs/tla/RecoveryReplayFull.tla`

Line 164: `journal[i].run /= -1` is meaningless since RunId is positive. Replace with:
```
\A i \in 1..Len(journal) :
    journal[i].run \in RunId  \* already guaranteed by TypeOK
```

Or simply remove the guard since TypeOK already constrains run to valid values.

**REPAIR STEP 3.2**: Fix `Sort` operator in `specs/tla/RecoveryReplayFull.tla`

Line 127 defines `Sort(s, less) == s` as identity. Either:
1. Import and use TLC's built-in `Sort` operator properly, or
2. Remove `to_replay` from `ReplayEvents` since the model doesn't actually need sorting for invariant checking

**REPAIR STEP 3.3**: Add missing INVARIANT declarations to `specs/tla/RecoveryReplayFull.cfg`

Add:
```
INVARIANT
    ReplaySeqOrder
    TailCausalAfterSnapshot
    OnlyIncompleteRuns
    NoResolvedReExecution
```

---

### DEFECT 4: TLA+ Model Not Verified by TLC (HIGH)

**REPAIR STEP 4.1**: Execute TLC and capture output

```bash
cd /home/lewis/src/velvet-ballistics
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla 2>&1 | tee tlc_recovery_full_output.txt
```

Expected: TLC reports 0 errors, all invariants satisfied.

---

## Execution Order

1. **Verus standalone files** (1.1 through 1.5) — create and verify independently
2. **Kani harness** (2.1) — create, compile, run with --no-unwind
3. **TLA+ fixes** (3.1 through 3.3) — fix spec, then run TLC (4.1)
4. **Re-run proof-review** with actual execution evidence

---

## Success Criteria

When all repairs are complete, the following must be true:
- `verus verification/verus/vb_rpch_*.rs` reports 0 errors for all 5 files
- `cargo kani -p vb_storage --harness kani_recovery_hydrate::hydrate_run_frame_precond_kani` completes without timeout
- `tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla` reports 0 errors
- All 22 BDD tests pass with actual test output evidence

---

*This guide was generated by proof-reviewer agent. Re-entry state: 5.*
