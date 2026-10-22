# Domain Contract Review v2 — vb-ceago (Local5090 Advisor)

**Status:** Advisory, awaiting root approval
**Date:** 2026-10-22
**Scope:** Runtime action authorization policy — domain contract guidance only

## Corrected Include Graph

Actual compiled include chain in `crates/vb_runtime/src/shard/lifecycle.rs`:
```
lifecycle.rs
├── lifecycle/execution_binding.rs       (mod)
├── lifecycle/registered_completion.rs   (mod)
├── lifecycle/chunk_003.rs
├── lifecycle/chunk_001.rs
├── lifecycle/chunk_002.rs
├── lifecycle/chunk_005_prepare.rs
├── lifecycle/chunk_005_drive_apply.rs
├── lifecycle/chunk_005.rs               ← prepare_awaiting_action, apply_awaiting_action
└── lifecycle/chunk_004.rs
```

Orphan (NOT compiled, 3090 initially edited this): `lifecycle/chunk_002_parts/chunk_002_drive_core.rs`

## Current Defects (Confirmed)

1. **RuntimePolicy is durability/admission, not authorization**: `RuntimePolicy::Strict/Journaled/Relaxed` controls artifact verification strictness and journal durability. It is not an operator authority that permits or denies action execution.

2. **Missing admission silent fallback**: `admit_run` with `RuntimePolicy::Relaxed` returns `Ok(RunAdmission)` without verifying an AcceptedArtifact exists. This is silent authority to bypass policy — not an explicit operator decision.

3. **Skips policy journaling when evidence.is_empty()**: In `apply_awaiting_action`, if `evidence.is_empty()`, the code calls `apply(run, RuntimeEvent::AwaitAction)` directly without appending any authorization decision event. The `evidence.is_empty()` branch is a non-evidenced drive step that still creates live runtime effects.

4. **No authorization decision at prepare_awaiting_action**: Current implementation computes retry capacity, normalizes ticket, records attempt, and generates `ActionScheduledTicket` journal event. It never consults or records an authorization decision from live policy.

5. **RunAdmission doesn't pin policy_digest**: `RunAdmission` stores `policy: RuntimePolicy` (durability level) but not the AcceptedArtifact's `policy_digest`. Replay cannot verify which policy version authorized the admission.

6. **No CurrentLivePolicy concept**: No structure tracks whether an operator has explicitly configured an authorization policy, or whether the system is in an unconfigured default state.

## Corrected Domain Contract

### 1. CurrentLivePolicy State

Define an explicit operator-configured live policy structure, separate from RuntimePolicy:

```
CurrentLivePolicy:
  Unconfigured — typed default deny; no actions permitted
  Configured {
    policy_digest: WorkflowDigest,      // BLAKE3 of operator's policy contract
    rule: BoundedPolicyRule,            // Permit / Deny / Suspend for action class
  }
```

Properties:
- `Unconfigured` means "operator has not explicitly authorized"; it is a typed deny, not a fallback to Relaxed.
- `Configured.rule` is bounded: only Permit, Deny, or Suspend. No open-ended policy language.
- `policy_digest` is computed from the operator's policy contract serialization, not substituted with the artifact's policy_digest or workflow digest.

### 2. EmergencyDenyOverlay

Separate operator control that overrides live policy:

```
EmergencyDenyOverlay:
  Clear — live policy decisions apply normally
  Denied — all actions denied regardless of live policy rule
```

Properties:
- Overlay can only be `Denied` by explicit operator decision.
- Once `Denied`, it cannot be cleared by updating live policy or admitting new runs.
- Requires explicit operator `Clear` decision to return to `Clear`.
- Applies to all runs and all action classes.

### 3. RuntimePolicy Remains Durability/Admission Only

Do not repurpose RuntimePolicy. It controls:
- `Strict`: require AcceptedArtifact, SyncAll before return
- `Journaled`: accept without artifact, queue events without sync barrier  
- `Relaxed`: no verification, testing only

RuntimePolicy never grants execution authorization.

### 4. Authorization Decision Record

Define explicit durable policy decision event, separate from ActionScheduledTicket:

```
PolicyDecisionEvent {
  decision_id: DecisionId,
  run: RunId,
  run_attempt: u16,
  step: StepIdx,
  action: ActionId,
  ticket: ActionTicket,
  input_value: SlotValue,          // actual input value read
  input_taint: Taint,             // actual taint of input
  policy_rule: BoundedPolicyRule, // rule that evaluated
  policy_digest: WorkflowDigest,  // policy version that evaluated
  decision: PolicyDecision,       // Permit / Denied / Suspended
  overlay: EmergencyDenyOverlay,  // overlay state at decision time
  decision_seq: EventSeq,         // durable sequence of this decision
}

PolicyDecision:
  Permit — action may execute
  Denied — action rejected, no pending ticket issued
  Suspended — action deferred, recoverable
```

The decision record binds: actual input value + taint, ticket, run/step/action/attempt identifiers, the policy rule that evaluated, the policy version (digest), the decision outcome, and the overlay state at decision time.

### 5. Authorization Flow at apply_awaiting_action

The actual compiled method is `apply_awaiting_action` in `lifecycle/chunk_005.rs`. The corrected flow:

```
apply_awaiting_action(run, state, ticket, evidence):
  1. Load CurrentLivePolicy + EmergencyDenyOverlay from shard state
  2. If CurrentLivePolicy is Unconfigured → Decision = Denied
  3. If EmergencyDenyOverlay is Denied → Decision = Denied
  4. Otherwise, evaluate live policy rule on (run, step, action, input) → Decision
  5. Construct PolicyDecisionEvent with all bindings (value, taint, ticket, policy_digest, overlay)
  6. Append PolicyDecisionEvent to journal BEFORE any other effects
  7. Branch on decision:
     a. Permit: construct ActionScheduledTicket event, append to same batch,
        insert run state, insert pending action ticket
     b. Denied: emit no ActionScheduledTicket, no pending action ticket
        (only PolicyDecisionEvent and transition to terminal/suspended state)
     c. Suspended: like Denied but recoverable terminal state
  8. Apply RuntimeEvent::AwaitAction (or appropriate terminal event)
```

Critical invariants:
- The PolicyDecisionEvent is always appended, regardless of decision outcome or evidence emptiness.
- ActionScheduledTicket is only appended when decision is Permit.
- The `evidence.is_empty()` branch is eliminated or refactored: no live effects without durable policy decision.
- No fake "AfterCommit" state before journal append.

### 6. RunAdmission Requires policy_digest

Change `RunAdmission` to include `policy_digest: WorkflowDigest` pinned from AcceptedArtifact at admission time. This enables replay to verify which policy version authorized the admission.

### 7. Replay Semantics

Historical replay must:
- Restore actual PolicyDecisionEvent records from journal
- Reconstruct run state from decision events, not re-evaluate live policy
- For retried effects: new execution requires new authorization decision under current policy
- Current EmergencyDenyOverlay=Denied cannot be loosened by reopening old runs

Replay cannot use `helper.clone()` as enforcement; it must reconstruct from durable decision records.

### 8. Executor/Pending Ticket API

To prevent old API from bypassing authorization:
- Define `execute_pending_action(ticket)` that consults the journal for the PolicyDecisionEvent bound to that ticket
- If no Permit decision found in journal → action is unauthorized → reject or error
- Old code paths that insert pending tickets directly must be migrated to go through apply_awaiting_action

If the backend has no executor port, document this as an honest domain gap.

## Behavioral Scenarios

### Scenario A: Admit then Deny

1. Run admitted under RuntimePolicy::Strict with AcceptedArtifact containing policy contract P1 (policy_digest=H(P1))
2. Operator updates CurrentLivePolicy to Configured{policy_digest=H(P2), rule=Deny} for action class X
3. Run executes, reaches action A of class X
4. apply_awaiting_action evaluates: live policy rule=Deny → Decision=Denied
5. PolicyDecisionEvent appended with policy_digest=H(P2), decision=Denied
6. No ActionScheduledTicket emitted, no pending ticket inserted
7. Run transitions to terminal failed or suspended

Expected: Action blocked despite run admission under P1. Admission ≠ action authorization.

### Scenario B: Denied Action — No Pending Ticket, Decision Survives Reopen

1. Action A evaluated → Decision=Denied, PolicyDecisionEvent appended
2. Run transitions to terminal state, removed from runtime_states
3. Operator attempts to manually resume/reopen run
4. Replay restores PolicyDecisionEvent from journal
5. No ActionScheduledTicket exists in journal for this action
6. Run cannot resume action execution

Expected: Reopening does not resurrect the denied action. The durable decision survives.

### Scenario C: Old Allowed Replay Under Current Deny — Zero Effects

1. Run previously admitted under policy P1 that permitted action A
2. Journal contains only historical events, no new drive steps
3. Operator sets CurrentLivePolicy to deny all
4. Replay restores historical run state from journal
5. Run reaches action A again (historical replay position)
6. No historical PolicyDecisionEvent for this retry exists
7. Replay reaches decision point: current policy=Deny, no prior Permit decision in journal for this position
8. Decision=Denied, no live action effects

Expected: Replay under deny yields zero effects for previously allowed actions. Past admission does not authorize future execution under new policy.

### Scenario D: Next Retry Blocked by Policy Update

1. Run admitted, action A attempted once (attempt 1), failed, run suspended
2. Policy updated to Deny action A
3. Operator retries run (attempt 2)
4. Run replays to action A, attempts execution
5. apply_awaiting_action evaluates: policy rule=Deny → Decision=Denied
6. No Permit decision recorded for attempt 2
7. Action fails with policy rejection, not execution error

Expected: Retry does not bypass policy update. Each execution requires fresh authorization.

### Scenario E: EmergencyDenyOverlay Cannot Clear by Policy Update

1. Operator sets EmergencyDenyOverlay=Denied
2. Run attempts action A → Decision=Denied (overlay)
3. Operator attempts to clear by updating CurrentLivePolicy to Permit
4. Policy rule changes but overlay remains Denied
5. Action A still denied
6. Operator must explicitly Clear overlay to resume

Expected: Overlay is independent, sticky, cannot be cleared indirectly.

## Exact Symbol/File Map

| Component | File | Symbol/Line |
|-----------|------|-------------|
| prepare_awaiting_action | `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` | fn prepare_awaiting_action (line ~1) |
| apply_awaiting_action | `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` | fn apply_awaiting_action (line ~110) |
| RuntimeJournalEvent | `crates/vb_runtime/src/journal/chunk_001.rs` | enum RuntimeJournalEvent |
| ActionScheduledTicket | `crates/vb_runtime/src/journal/chunk_001.rs` | variant ActionScheduledTicket |
| ActionTicket | `crates/vb_core/src/action/ticket.rs` | struct ActionTicket |
| RunAdmission | `crates/vb_runtime/src/admission/parts/chunk_002_records.rs` | struct RunAdmission |
| admit_run | `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs` | fn admit_run |
| AcceptedArtifact | `crates/vb_storage/src/admission.rs` | struct AcceptedArtifact |
| RuntimePolicy | `crates/vb_core/src/policy.rs` | enum RuntimePolicy |
| EvidenceCollector | `crates/vb_runtime/src/engine/types.rs` | struct EvidenceCollector |

## First Vertical Slice (Implementation)

A minimal viable implementation slice would address:

1. **Define types**: CurrentLivePolicy, EmergencyDenyOverlay, BoundedPolicyRule, PolicyDecision, DecisionId
2. **Extend RunAdmission**: Add `policy_digest: WorkflowDigest` field
3. **Update admit_run**: Pass policy_digest from AcceptedArtifact to RunAdmission
4. **Add PolicyDecisionEvent variant**: To RuntimeJournalEvent enum
5. **Implement shard policy state**: Fields for CurrentLivePolicy and EmergencyDenyOverlay in Shard struct
6. **Wire into apply_awaiting_action**: Evaluate policy, emit PolicyDecisionEvent before ActionScheduledTicket
7. **Refactor evidence.is_empty() branch**: Ensure policy decision is always emitted regardless of evidence
8. **Update replay path**: Reconstruct state from PolicyDecisionEvent records

This slice addresses the core contract: explicit authorization decisions are durable, bound to actual runtime values, and survive replay.
