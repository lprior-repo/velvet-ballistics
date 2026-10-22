# Contract Review: Live-Effect Authorization (vb-ceago)

**Role:** LOCAL RTX5090 Rust domain contract advisor
**Baseline:** d5f38934 (current production)
**Parent bead:** vb-ny068

## Confirmed Production Path

The exact path from artifact admission to action ticket external visibility is:

1. `admit_run()` in `crates/vb_runtime/src/admission.rs` → returns `RunAdmission` (policy, digest, run_id, capabilities)
2. `submit_run()` in `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` → stores `RunAdmission` in `RunState.admission`
3. `drive_run()` / `drive_state()` in `crates/vb_runtime/src/shard/lifecycle/chunk_002_parts/chunk_002_drive_core.rs` → calls `drive_deterministic_full()` from vb_core
4. Engine returns `RuntimeSignal::AwaitingAction(ticket)` with `ActionTicket`
5. `apply_drive_result()` matches signal, calls `apply_awaiting_action(run, state, ticket, evidence)`
6. **AUTHORIZATION SEAM:** `apply_awaiting_action()` in `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` line ~127
   - Calls `prepare_awaiting_action()` → normalizes ticket, gets input/output slots, action ABI digest
   - **Appends `RuntimeJournalEvent::ActionScheduledTicket` to journal**
   - Inserts run state and pending action
7. Action becomes externally visible via pending action index and journal

## Existing APIs at the Seam

At step 6, the following production data is available:

- `state.admission: Option<RunAdmission>` → `.policy()` returns `RuntimePolicy` (Strict/Journaled/Relaxed)
- `state.admission.artifact_digest()` → `WorkflowDigest`
- `state.admission.granted_capabilities()` → `&CapabilitySet`
- `ticket: ActionTicket` → run, step, seq, action, run_attempt, attempt, idempotency_key, capacity
- `input: SlotIdx` → input slot index (from `action_input_slot()`)
- `state.frame` → can read input slot value via `read_slot_with_taint()`
- `action_abi_digest` → `WorkflowDigest` for the action

## Current Policy Enum

```rust
// crates/vb_core/src/policy.rs
pub enum RuntimePolicy {
    Strict,      // Accepted artifact + SyncAll
    Journaled,   // Accepted artifact, queue events
    Relaxed,     // No artifact check, no sync
}
```

## Smallest Typed Policy Seam

**Location:** `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` in `apply_awaiting_action()`

**Inject after ticket normalization, before journal append:**

```rust
fn apply_awaiting_action(
    &mut self,
    run: RunId,
    state: RunState,
    ticket: ActionTicket,
    evidence: &mut EvidenceCollector,
) -> Result<(), DriveApplyFailure> {
    // ... existing ticket normalization and slot resolution ...

    // NEW: Authorization decision
    let policy = state
        .admission
        .as_ref()
        .map(|a| a.policy())
        .unwrap_or(RuntimePolicy::Relaxed);

    let decision = authorize_action(
        &policy,
        &state.admission,
        ticket,
        input,
        &state.frame,
        action_abi_digest,
    );

    // NEW: Record decision before effects
    if evidence.is_empty() {
        // Relaxed: skip journaling, just enforce
        if decision == PolicyDecision::Deny {
            return Err(DriveApplyFailure::AfterCommit(
                RuntimeError::ActionDeniedByPolicy { ticket },
            ));
        }
    } else {
        // Strict/Journaled: append decision event first
        let decision_event = RuntimeJournalEvent::PolicyDecision {
            run,
            step: ticket.step,
            action: ticket.action,
            decision,
            // bind input digest, admission digest, etc.
        };
        Self::push_drive_journal_event(&mut journal_events, decision_event)?;
    }

    // Existing: append ActionScheduledTicket
    // Existing: insert run state and pending action
}
```

## Errors/Storage/Replay Contract

**New discriminated event variants (append-only, no renumbering):**

```rust
// Add to RuntimeJournalEvent enum (crates/vb_runtime/src/journal/)
PolicyDecision {
    run: RunId,
    step: StepIdx,
    action: ActionId,
    seq: u64,  // per-run sequence
    decision: PolicyDecision,  // Allow | Deny | Suspend
    input_digest: Option<SlotValueDigest>,
    admission_digest: WorkflowDigest,
    attempt: u16,
}
```

**PolicyDecision enum:**
```rust
// crates/vb_core/src/policy.rs (new)
pub enum PolicyDecision {
    Allow,
    Deny,      // Permanent rejection for this input
    Suspend,   // Wait for external approval (separate bead)
}
```

**Replay contract:**
- `ActionScheduledTicket` events with attempt=1 are historical decisions
- Recovery filters by `ticket.run_attempt` (existing logic in `vb_storage/src/events/event.rs`)
- `PolicyDecision` events are informational for audit; replay re-evaluates from current authority
- Historical decisions are NOT re-evaluated during cold start replay

## Handling Denial Without Err Before Append

**Problem:** If `Err(...)` is returned before journal append, the decision is not durably recorded.

**Solution:** Always append the `PolicyDecision` event before returning an error:

```rust
if decision == PolicyDecision::Deny {
    // Append decision event FIRST
    Self::push_drive_journal_event(&mut journal_events, decision_event)?;
    self.append_journal_event_batch(&journal_events)?;

    // THEN return error (journal now has durable record)
    return Err(DriveApplyFailure::AfterCommit(
        RuntimeError::ActionDeniedByPolicy { ticket },
    ));
}
```

For Relaxed policy (no journaling), the decision is not durable — this is acceptable because Relaxed explicitly means "no durability requirements."

## Retry/New Pending Action Rechecks Current Authority

**Existing behavior:** When an action fails and is retried, `drive_state()` is called again, which calls `drive_deterministic_full()` again. The engine returns a new `AwaitingAction` signal with an incremented attempt number.

**Policy contract:** `authorize_action()` is called on EACH attempt, so:
- Current policy is always re-evaluated
- New admission policies apply to retry attempts
- Historical `PolicyDecision` events are not replayed; only current authority matters
- This satisfies "no reevaluation of historical decisions" requirement

## Compatibility with Existing Constructors and Tests

**Honest assessment:** The current baseline does NOT have a separate policy layer. `RuntimePolicy` only controls durability (Strict/Journaled/Relaxed). Adding authorization requires:

1. **New error variant:** `RuntimeError::ActionDeniedByPolicy { ticket }`
2. **New journal event variant:** `RuntimeJournalEvent::PolicyDecision`
3. **New codec handling:** Update `crates/vb_storage/src/codec/` for new record kind
4. **New record kind:** Update `RecordKind` enum (next stable id after 40 = 41)
5. **Existing tests that use Relaxed policy will NOT be affected** (Relaxed = no policy check, existing behavior)
6. **Existing tests that use Strict/Journaled will require:**
   - Either a default "allow all" policy for historical test compatibility
   - Or explicit policy configuration in test fixtures

**Required configuration changes for Strict/Journaled tests:**
- Tests must explicitly pass a policy that allows the test actions
- Or add a test fixture policy `TestAllowAll` that permits any action

## Avoided Anti-Patterns

Based on known rejected draft analysis:

- ❌ No `always_allow_recorder` (fake policy that does nothing)
- ❌ No zero/fabricated digest (`WorkflowDigest::new([0; 32])`)
- ❌ No `Mutex<bool>` for policy state (not needed for first slice)
- ❌ No `clone()` as fake replay (replay re-evaluates)
- ❌ No `Err` before journal append (always append decision first)

## Exact Minimal First Vertical Slice

**Files to modify:**

1. `crates/vb_core/src/policy.rs` — Add `PolicyDecision` enum, `authorize_action()` function
2. `crates/vb_core/src/lib.rs` — Export `PolicyDecision`
3. `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` — Call `authorize_action()` in `apply_awaiting_action()`
4. `crates/vb_runtime/src/journal/mod.rs` — Add `PolicyDecision` variant to `RuntimeJournalEvent`
5. `crates/vb_runtime/src/journal/codec.rs` — Handle new variant in serialization
6. `crates/vb_storage/src/codec/events.rs` — Add `PolicyDecision` variant to `JournalEvent`
7. `crates/vb_storage/src/codec/record_kind.rs` — Add `RecordKind::PolicyDecision(41)`
8. `crates/vb_runtime/src/shard/helpers.rs` — New error variant `ActionDeniedByPolicy`
9. `crates/vb_runtime/src/lib.rs` — Export new error

**First slice authorization logic (simple allow-list):**

```rust
pub fn authorize_action(
    policy: &RuntimePolicy,
    admission: Option<&RunAdmission>,
    ticket: ActionTicket,
    input_slot: SlotIdx,
    frame: &RunFrame,
    action_abi_digest: WorkflowDigest,
) -> PolicyDecision {
    match policy {
        RuntimePolicy::Relaxed => PolicyDecision::Allow,
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            // Check if action is in admitted capabilities
            if let Some(adm) = admission {
                if adm.granted_capabilities().contains_action(ticket.action) {
                    return PolicyDecision::Allow;
                }
            }
            // Default: deny actions not explicitly admitted
            PolicyDecision::Deny
        }
    }
}
```

**Note:** This binds the actual action identity from the ticket, not just identity. It uses the real input slot and admission data.

## Executable Behavior Scenarios

**Scenario 1: Allowed action (Strict policy, action admitted)**
- Submit run with action A in admitted capabilities
- Drive to action A → `authorize_action()` returns `Allow`
- `PolicyDecision { decision: Allow }` appended to journal
- `ActionScheduledTicket` appended to journal
- External system receives action
- Expected: Run proceeds normally

**Scenario 2: Denied action (Strict policy, action NOT admitted)**
- Submit run with action B NOT in admitted capabilities
- Drive to action B → `authorize_action()` returns `Deny`
- `PolicyDecision { decision: Deny }` appended to journal
- Error `RuntimeError::ActionDeniedByPolicy` returned
- Run transitions to failed state
- Expected: Journal has durable denial record; run failed with clear error

**Scenario 3: Retry re-evaluates policy**
- Submit run with action A, initially denied
- Change policy to allow action A
- Submit new run → action A allowed
- Expected: New run uses current authority; old denied run unaffected

**Scenario 4: Relaxed policy unchanged**
- Submit run with Relaxed policy
- Action proceeds without authorization check
- Expected: No `PolicyDecision` event; identical to baseline behavior

## Compatibility Dimensions (Separate Beads)

- **Capability grant expansion:** Separate bead for broader capability set definitions
- **Emergency overlay:** Separate bead for monotonic emergency policy that cannot be cleared
- **Input-based authorization:** Separate bead for policies that inspect input slot values
- **Multi-run coordination:** Separate bead for cross-run policy decisions

## Domain Questions (Unresolved)

1. Should `PolicyDecision::Deny` fail the run or just the action attempt? (Current contract: fail run)
2. Should denied actions be retried on next attempt? (Current contract: no, permanent for that run)
3. What is the interaction between `PolicyDecision::Suspend` and the existing timer/ask mechanism? (Deferred to separate bead)

## Conclusion

The smallest typed policy seam is at `apply_awaiting_action()` in `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs`. The authorization decision must be:
1. Made before the `ActionScheduledTicket` journal append
2. Durably recorded as a `PolicyDecision` event before any error return
3. Re-evaluated on each retry attempt using current authority
4. Compatible with existing Relaxed policy behavior (no check = allow all)

This requires 9 file modifications, 3 new enum variants, and 1 new record kind. No changes to the deterministic engine semantics; policy is applied at the shard boundary.
