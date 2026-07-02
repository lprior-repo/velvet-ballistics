# vb-pcu4h Implementation — Assert pending-action recovery fields exactly

## Scope

Replace fuzzy `.iter().any(|entry| entry.step == X && entry.action == Y)`
matchers with struct-level `assert_eq!` on whole `Vec<RecoveredPendingAction>`
in three PRIMARY test targets in
`crates/vb_storage/src/recovery/replay/summary/tests.rs`.

## Production Code Mutation

**None.** Bead is bounded to test-only edits per `delivery-scope.jsonl`.

## Test Edits

### Import (line 3)

```rust
// before
use crate::recovery::types::RecoveryTerminalState;
// after
use crate::recovery::types::{RecoveredPendingAction, RecoveryTerminalState};
```

### Test 1 — `unresolved_action_marks_pending_action_recovery_unsupported` (lines 436-463)

```rust
// before (fuzzy + Err-silent-pass via matches!)
assert!(
    matches!(seed, Ok(recovered) if recovered.pending_actions.iter().any(|entry|
        entry.step == StepIdx::new(3) && entry.action == ActionId::new(9)
    ) && recovered.unsupported.pending_actions)
);

// after (exact Vec equality + .expect() + boolean assertion preserved)
let recovered = recover_runtime_frame_seed_from_events(&events)
    .expect("schedule-only event must produce a recoverable seed");

assert_eq!(
    recovered.pending_actions,
    vec![RecoveredPendingAction {
        step: StepIdx::new(3),
        action: ActionId::new(9),
    }],
    "schedule-only event must surface exactly the scheduled pending action"
);
assert!(
    recovered.unsupported.pending_actions,
    "schedule-only event must mark pending-action recovery unsupported"
);
```

### Test 2 — `action_scheduled_ticket_advances_max_slot_and_step_dimensions` (lines 674-681)

```rust
// before
assert!(
    seed.pending_actions
        .iter()
        .any(|entry| { entry.step == StepIdx::new(5) && entry.action == ActionId::new(11) }),
    "ActionScheduledTicket must remain pending until completion/abandon",
);

// after
assert_eq!(
    seed.pending_actions,
    vec![RecoveredPendingAction {
        step: StepIdx::new(5),
        action: ActionId::new(11),
    }],
    "ActionScheduledTicket must remain pending until completion/abandon",
);
```

### Test 3 — `crash_after_schedule_then_recover_hydrates_resume_queue` (lines 794-804)

```rust
// before
assert!(
    seed.pending_actions
        .iter()
        .any(|entry| { entry.step == StepIdx::new(6) && entry.action == ActionId::new(17) }),
    "crashed-while-pending action must surface in the resume queue",
);

// after
assert_eq!(
    seed.pending_actions,
    vec![RecoveredPendingAction {
        step: StepIdx::new(6),
        action: ActionId::new(17),
    }],
    "crashed-while-pending action must surface in the resume queue",
);
```

## Pattern Mirrored

Strong-exhaustive pattern from `recovery_type_tests.rs:118-126`:

```rust
let pending = RecoveredPendingAction {
    step: StepIdx::new(7),
    action: ActionId::new(99),
};
assert_eq!(pending.step, StepIdx::new(7));
assert_eq!(pending.action, ActionId::new(99));
```

`RecoveredPendingAction` already derives `PartialEq, Eq`
(`crates/vb_storage/src/recovery/types.rs:644`), so direct `vec![]` equality
is sound.

## Diff Stats

```
M crates/vb_storage/src/recovery/replay/summary/tests.rs
1 file changed, 25 insertions(+), 13 deletions(-)
```

## Out of Scope (NOT Modified Per Bead)

- `crates/vb_storage/src/recovery/recovery_unit_tests.rs:314-351`
  (`recovery_cannot_resume_state_classifies_pending_action`) — hand-built
  seed test, already exhaustive via direct `vec![RecoveredPendingAction {
  ... }]`. Per bead instruction: leave alone.
- `crates/vb_storage/src/recovery_type_tests.rs:118-126` — already
  exhaustive; the pattern we mirror.
- `crates/vb_runtime/tests/recovery_hydration_tests.rs:1899-1905` and
  `:2031-2037` — SECONDARY scope per `delivery-scope.jsonl`; deferred to
  contract agent decision.
- All production source files (`recovery/types.rs`, `recovery/replay/summary/derive.rs`,
  `recovery/replay/summary/accumulator.rs`).

## Verification Evidence

| Command | Status | Notes |
|---------|--------|-------|
| `cargo test -p vb_storage --lib recovery` | PASS | 250 passed, 1280 filtered |
| `cargo test -p vb_storage --lib -- unresolved_action_marks_pending_action_recovery_unsupported action_scheduled_ticket_advances_max_slot_and_step_dimensions crash_after_schedule_then_recover_hydrates_resume_queue` | PASS | 3 passed (the three strengthened tests) |
| `cargo test -p velvet-ballistics-workspace-tests` | MIXED | See classification below |

### Workspace Tests Classification

The single workspace_tests failure
(`given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`
at `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs:1466`)
is a **pre-existing repo-wide failure** in strict runtime admission tests
that check source-code string presence (`impl AcceptedArtifactStore for
AlwaysPresentArtifactStore`). It is **completely unrelated** to recovery
pending actions.

Classification: `BLOCK_GLOBAL` prerequisite repair — not introduced by this
bead. `jj diff --summary` shows the bead only modifies
`crates/vb_storage/src/recovery/replay/summary/tests.rs`. The admission
test failure exists on parent `lzmznkmm` (untouched by this commit). The
bead's gate explicitly required:

- `cargo test -p vb_storage --lib recovery passes (3 strengthened tests)` — PASS
- `cargo test -p workspace_tests passes` — DEFERRED with blocker recorded

Per Holzman doctrine (`scope_aware_blocking`), already-present repo-wide
failures are `BLOCK_GLOBAL` prerequisite repair with proof before
advancement, not a defect in this bead's delivery scope.

## Power-of-Ten Compliance

| Rule | Status | Note |
|------|--------|------|
| 1. Simple control flow | SATISFIED | Tests remain flat assertions |
| 2. Fixed loop bounds | N/A | No loops introduced |
| 3. No post-init alloc | N/A | No allocations added; `vec![]` is compile-time |
| 4. Functions fit on one page | SATISFIED | Tests unchanged in length beyond assertion substitution |
| 5. Assertion density | IMPROVED | Vec-equality is denser than field-pair `.any()` |
| 6. Smallest scope | SATISFIED | Local-only edits |
| 7. Checked returns | IMPROVED | Test 1 now `.expect()`s the seed instead of matching silently |
| 8. Limited macros | N/A | No macros touched |
| 9. Restricted pointer use | N/A | No pointers |
| 10. Warnings mandatory | SATISFIED | No new warnings expected (test code) |

## Zero-Panic Rules

Tests legitimately use `assert_eq!` — this is the canonical Holzman
exception for test code (per skill rule 5: "assert-style macros are
forbidden except tests, benches, build scripts, or process-start invariant
failure with diagnostics"). Production code remains panic-free.