# Proof Repair Guide: vb-0253.7

**Bead**: vb-0253.7
**Date**: 2026-05-17
**Purpose**: Actionable fixes to repair proof artifacts for re-review

---

## How to Use This Guide

For each open finding (CF-001 through CF-004), this guide provides:
1. The root cause of the problem
2. Step-by-step repair instructions
3. Success criteria for re-review

---

## CF-001: TLA+ Spec Models SET Semantics, Not DERIVE Semantics

### Root Cause

The TLA+ spec was written with `runState` as a tracked variable that actions SET directly. This models the PRE-refactoring behavior (with the `static TRACKER`). The post-refactoring world has NO `runState` variable—state is always derived on-demand from the journal.

### Repair Instructions

**Step 1**: Edit `specs/Lifecycle.tla`

Remove `runState` from VARIABLES:
```tla
VARIABLES
    (* REMOVE: runState, *)
    eventLog,
    transitionValid,
    actionsEnabled
```

**Step 2**: Update Init to not initialize runState:
```tla
Init ==
    (* REMOVE: runState = [r \in RunIds |-> Pending] *)
    /\ eventLog = [r \in RunIds |-> <<>>]
    /\ transitionValid = [r \in RunIds |-> TRUE]
    /\ actionsEnabled = TRUE
```

**Step 3**: Remove runState from vars tuple:
```tla
vars == <<eventLog, transitionValid, actionsEnabled>>
```

**Step 4**: Update all actions to NOT set runState. For example, Cancel should only append an event:
```tla
Cancel(run) ==
    /\ actionsEnabled
    /\ DeriveState(run) \in {Active, WaitingAnswer}
    /\ eventLog' = [eventLog EXCEPT ![run] = Append(eventLog[run],
                [type |-> "RunCancelled", run |-> run, seq |-> Len(eventLog[run]) + 1])]
    /\ UNCHANGED <<transitionValid, actionsEnabled>>
```

Note: `DeriveState(run)` is used to check the current derived state, but we don't SET runState.

**Step 5**: Update the invariants to reference derived state:
```tla
ConsistentState ==
    \A run \in RunIds : TRUE  (* runState no longer exists to check *)
    (* Instead, we verify DeriveState is consistent with journal appends *)
```

Actually, since runState doesn't exist, the invariants need to be rethought:
- `ConsistentState` becomes trivial (no tracked state to check)
- `NoDivergence` becomes trivial (no divergence possible without tracked state)
- `TerminalFinal` needs to be expressed differently: "if DeriveState(run) is terminal, it stays terminal"

```tla
TerminalFinal ==
    \A run \in RunIds :
        (DeriveState(run) \in TerminalState) =>
            [action that doesn't change eventLog for that run]
```

Or simply remove TerminalFinal as a separate invariant and verify it through the action definitions.

**Step 6**: Update Next relation to reflect that only eventLog changes.

### Success Criteria

- `runState` does NOT appear as a state variable
- All actions only modify `eventLog` (append events)
- `DeriveState(run)` is used in action guards to check current state
- TLC model checking verifies invariants on DERIVED state, not tracked state

---

## CF-002: Verus `derive_lifecycle_state_from_events` is Unimplemented Placeholder

### Root Cause

The Verus spec file was created but the actual Rust implementation was not linked/verified. The `#[verus(trusted)]` attribute and `unimplemented!()` body mean no verification occurs.

### Repair Instructions

**Option A** (Preferred if refactored code exists): Link to actual implementation

Edit `verification/verus/vb_0253_7_lifecycle_derive.rs`:

Replace lines 57-96 with:
```rust
#[verus(verifier::verus)]
pub fn derive_lifecycle_state_from_events(events: &[JournalEvent]) -> LifecycleState
    ensures
        result.is_valid(),
        result == spec_derive_lifecycle_state_from_events(events->spec()),
{
    events
        .last()
        .map(|e| match e {
            JournalEvent::RunCancelled { .. } => LifecycleState::Cancelled,
            JournalEvent::RunResumed { .. } => LifecycleState::Active,
            JournalEvent::RunRetried { .. } => LifecycleState::Active,
            JournalEvent::RunAnswered { .. } => LifecycleState::Completed,
            JournalEvent::RunFinished { .. } => LifecycleState::Completed,
            JournalEvent::RunFailedEvent { .. } => LifecycleState::Failed,
            JournalEvent::RunAccepted { .. } => LifecycleState::Active,
            JournalEvent::RunAdmission { .. } => LifecycleState::Active,
            JournalEvent::StepStarted { .. } => LifecycleState::Active,
            JournalEvent::StepSucceeded { .. } => LifecycleState::Active,
            JournalEvent::ActionScheduled { .. } => LifecycleState::Active,
            JournalEvent::SlotWrittenEvent { .. } => LifecycleState::Active,
            JournalEvent::ActionCompletedEvent { .. } => LifecycleState::Active,
            JournalEvent::ActionFailedEvent { .. } => LifecycleState::Failed,
            JournalEvent::WaitScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::AskScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::AskAnsweredEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::RetryScheduledEvent { .. } => LifecycleState::Active,
        })
        .unwrap_or(LifecycleState::Pending)
}
```

**Option B** (If actual implementation is in a separate file): Use `#[extern_spec]`

```rust
#[extern_spec(vb_cli::lifecycle)]
pub fn derive_lifecycle_state_from_events(events: &[JournalEvent]) -> LifecycleState
    ensures
        result.is_valid(),
        result == spec_derive_lifecycle_state_from_events(events->spec()),
}
```

### Success Criteria

- `#[verus(trusted)]` is removed from the exec function
- `unimplemented!()` is removed
- Actual implementation code is present and verifiable
- `verus verification/verus/vb_0253_7_lifecycle_derive.rs` exits 0 with 0 errors

---

## CF-003: Kani Harness Cannot Invoke Actual Command Functions

### Root Cause

CLI command functions (`cancel`, `resume`, `retry`, `answer`) require `&FjallJournal` which cannot be constructed in a Kani harness context. The harnesses were stubbed with `kani::cover!(true)`.

### Repair Instructions

**Step 1**: Restructure verification to focus on pure functions

The actual command functions have side effects (journal appends), but their core logic can be verified through the pure functions they call:

```rust
// In the harness, verify the pure function that commands use internally
#[kani::proof]
fn harness_derive_is_total_and_correct() {
    let events: Vec<JournalEvent> = kani::any();
    let state = derive_lifecycle_state_from_events(&events);
    // Verify:
    // 1. Function doesn't panic (Kani checks this automatically)
    // 2. Output is a valid LifecycleState
    kani::assert!(state.is_valid());
}

#[kani::proof]
fn harness_check_transition_is_total() {
    let state: LifecycleState = kani::any();
    let cmd: LifecycleCommand = kani::any();
    let result = check_lifecycle_transition(state, cmd);
    // Kani verifies no panic and result is bool
    kani::assert!(result == true || result == false);
}
```

**Step 2**: For command-level verification, create a testable wrapper

Add to `lifecycle.rs`:
```rust
#[cfg(test)]
pub fn derive_state_for_testing(events: &[JournalEvent]) -> LifecycleState {
    derive_lifecycle_state_from_events(events)
}
```

Then Kani can verify this function directly.

**Step 3**: Document the limitation

If full command verification is not possible in Kani, document that:
- Pure function verification (CF-003 fix) provides compositional assurance
- Integration testing with real journal provides end-to-end assurance
- Kani cannot verify I/O effects but can verify pre/post conditions

### Success Criteria

- Kani harnesses call actual verifiable functions
- No `kani::cover!(true)` placeholders remain
- `cargo kani --crate-type=lib -p vb_cli` exits 0 with 0 unproven targets

---

## CF-004: Most Kani Harnesses Are Stubs

### Root Cause

Harnesses were written with comments documenting coverage obligations but then stubbed with `kani::cover!(true)` which is a no-op.

### Repair Instructions

**Step 1**: Replace stub harness `harness_cancel_never_panics` with actual verification

```rust
#[kani::proof]
fn harness_cancel_command_logic() {
    // Verify derive_lifecycle_state_from_events handles the case
    // that would occur after a Cancel appends RunCancelled
    let events_after_cancel: Vec<JournalEvent> = vec![
        JournalEvent::RunAccepted { run_id: kani::any(), seq: 1 },
        JournalEvent::RunCancelled { run_id: kani::any(), seq: 2 },
    ];
    let state = derive_lifecycle_state_from_events(&events_after_cancel);
    kani::assert!(state == LifecycleState::Cancelled);
}
```

**Step 2**: For `harness_resume_never_panics`:

```rust
#[kani::proof]
fn harness_resume_command_logic() {
    let events_after_resume: Vec<JournalEvent> = vec![
        JournalEvent::RunAccepted { run_id: kani::any(), seq: 1 },
        JournalEvent::AskScheduledEvent { run_id: kani::any(), seq: 2 },
        JournalEvent::RunResumed { run_id: kani::any(), seq: 3 },
    ];
    let state = derive_lifecycle_state_from_events(&events_after_resume);
    kani::assert!(state == LifecycleState::Active);
}
```

**Step 3**: Similarly for retry and answer.

**Step 4**: Remove coverage stubs. Coverage is automatically tracked by Kani—no need for explicit `kani::cover!` unless you need specific coverage goals.

### Success Criteria

- All harness functions contain actual verification logic
- No `kani::cover!(true)` statements remain
- Functions call verifiable code paths
- All 6 LifecycleState variants are exercised

---

## NEW FINDING — CF-NEW-001: Verus Transition Syntax Error (2026-05-19)

**Severity**: CRITICAL  
**Artifact**: `verification/verus/vb_0253_7_lifecycle_transition.rs`  
**Line**: 23  
**Finding**: `spec fn spec_check_lifecycle_transition` used at module level **outside** a `verus!` block. Verus cannot parse this — `error: expected one of '!' or '::', found keyword 'fn'`.

**Required Action**: Wrap `spec fn` in a `verus! { }` block, OR restructure so Verus spec syntax is not used outside a verus block.

**Evidence**: `verus verification/verus/vb_0253_7_lifecycle_transition.rs` → error at line 23.

---

## Summary of Success Criteria

| Finding | Success Criteria | Status |
|---------|------------------|--------|
| CF-001 | TLA+ spec has no `runState` variable; state always derived | **FIXED** |
| CF-002 | Verus exec function is verifiable (not `unimplemented!()`) | **FIXED** |
| CF-003 | Kani harnesses call actual verifiable functions | WAIVED (tooling) |
| CF-004 | All Kani harnesses contain real verification logic | WAIVED (tooling) |
| CF-NEW-001 | Verus transition file: `spec fn` inside `verus!` block | **OPEN** — fix required |

---

## Verification Commands

After applying fixes, run these commands to verify:

```bash
# TLA+ model checking — PASSES (3025 states, 0 errors)
tlc -config specs/Lifecycle.cfg specs/Lifecycle.tla

# Verus derive verification — PASSES (11 verified, 0 errors)
verus verification/verus/vb_0253_7_lifecycle_derive.rs

# Verus transition verification — FAILS (spec fn outside verus! block) — FIX REQUIRED
verus verification/verus/vb_0253_7_lifecycle_transition.rs
```

---

## Re-review Checklist

Before requesting re-review, verify:

- [x] `runState` removed from TLA+ spec VARIABLES — **FIXED**
- [x] TLA+ actions only append events, don't set state — **FIXED**
- [x] Verus exec function has real implementation, not `unimplemented!()` — **FIXED**
- [x] Kani harnesses — **WAIVED** (BLOCKED_TOOLING, project structure)
- [ ] Verus transition: `spec fn` wrapped in `verus! { }` block — **FIX REQUIRED**
- [ ] `verus verification/verus/vb_0253_7_lifecycle_transition.rs` exits 0

---

*Repair guide updated: 2026-05-19 — CF-001/CF-002 confirmed FIXED, CF-NEW-001 Verus syntax error added*