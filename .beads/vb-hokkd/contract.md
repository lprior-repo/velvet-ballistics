# Policy Dispatch and Control Seams (vb-hokkd)

Source-grounded Rust contract connecting the accepted pure LivePolicy foundation to the actual live effect boundary, with durable policy decisions and monotonic emergency deny.

## Current State (as of main0434f64f)

The runtime has LivePolicy and DecisionRecord types but **no runtime enforcement**:

- `crates/vb_runtime/src/runtime/registered_executor.rs:execute_registered_action()` calls `registered_context()` → verifies `RegisteredExecutionMode::Live` → `adapter.execute()`. **No policy evaluation between context verification and effect dispatch.**
- `reconcile_registered_action()` calls `adapter.reconcile()` and remains effect-free. Correct.
- `crates/vb_runtime/src/policy.rs:LivePolicy` is a plain owned evaluator with `configure()`, `activate_emergency_deny()`, `evaluate()`. **Not wired into any runtime struct.**
- `crates/vb_runtime/src/runtime.rs:Runtime` has `policy: vb_core::policy::RuntimePolicy` (durability: Strict/Journaled/Relaxed) but **no LivePolicy field**.
- `crates/vb_runtime/src/journal/chunk_001.rs:RuntimeJournalEvent` enum has **no policy decision variant**.
- `crates/vb_runtime/src/shard/lifecycle/registered_completion.rs:registered_action_context()` returns `RegisteredActionContext` with the actual admitted `AcceptedArtifact` — the source for the admitted policy pin.

The in-memory LivePolicy is not durable. Emergency deny is lost on close/reopen. Historical policy decisions are not persisted. Master77.3 correction is binding: deterministic transition reconstructs bounded outbox intents under pinned deterministic context; current live policy MUST NOT change historical outbox identity or replay. Authorization occurs at new EFFECT dispatch, immediately before adapter.execute.

## Contract

### LivePolicy is Actor-Owned Per-Runtime, Configured Externally

```rust
impl Runtime {
    /// Runtime owns a single LivePolicy instance. Plain owned, no Arc/Mutex.
    /// LivePolicy::evaluate() takes no global state or ambient time/network/config reads.
    pub fn live_policy(&self) -> &LivePolicy;
    pub fn live_policy_mut(&mut self) -> &mut LivePolicy;
}
```

Current default: `LivePolicy::Unconfigured` → all actions denied. Configure one declared action for the existing first-wedge registered executor using actual `PolicyRule`/`PolicyDigest`.

### Policy Evaluation at Effect Dispatch Boundary

Master77.3: schedule ≠ authorize. Re-evaluate at every retry or new effect attempt.

**Modified execution path:**

```text
execute_registered_action(adapter, run, step, grant)
    → registered_context()          [existing: ticket, artifact, value, output, mode]
    → policy.evaluate(ticket, admission_pin, input_bytes, taint)  [NEW]
    → journal.append_sequenced(PolicyDecisionEvent{record})       [NEW]
    → if outcome != Allowed: return Err(PolicyDenied{record})     [NEW]
    → adapter.execute(grant, ticket, value)                       [existing]
    → enqueue_registered_completion(...)                           [existing]
```

Reconcile remains effect-free:

```text
reconcile_registered_action(adapter, run, step, grant)
    → registered_context()
    → adapter.reconcile(grant, ticket, value)  [read-only, no policy re-eval]
    → enqueue_registered_completion(...)
```

### Admission Pin Source

The admitted policy pin is the **accepted artifact's policy digest** from `RegisteredActionContext.artifact`:

```rust
let pinned_admission = PinnedAdmissionPolicy::Artifact(
    PolicyDigest::from_bytes(context.artifact.policy_digest.as_bytes())
);
```

Integration requirement: `vb-h7rex` is repairing `AcceptedArtifact.policy_digest` admission/recovery. The API is not yet accepted. This contract requires that `vb-h7rex` is completed and merged before policy evaluation can use the actual admitted pin. Do not silently substitute `artifact.source_digest`, workflow digest, or invent pin values.

### Durable DecisionRecord Before Effect

**Master77.3 binding:** full DecisionRecord must be appended successfully BEFORE adapter.execute. Journal failure or denied/suspended outcome → zero adapter.execute/reconcile calls.

The storage child vb-s4hfc (tag38, frozen production1a231a9f) provides the full stable storage API for `JournalEvent::PolicyDecisionEvent` with wire format for `DecisionRecord`. Source: `/home/lewis/src/isoloated/velvet-ballistics-vb-s4hfc-local5090-full-record`.

Admission tags39/41 are reserved; do not reuse them for this contract.

### Emergency Deny Scope and Durability

- In-memory `LivePolicy.emergency_deny_active: bool` survives for the process lifetime.
- Process restart resets emergency deny to inactive. This is correct per master77.11: emergency deny is a monotonic **policy layer** evaluated at effect dispatch, not a durable global metadata record.
- Activation must be journaled as a `PolicyControlEvent { kind: EmergencyDenyActivated, timestamp }` for audit. Deactivation is not part of the first slice (see follow-ups).
- If a durable global runtime control is later required, it uses the existing production trait/storage seam: `RuntimeJournal` trait with a new `RuntimeJournalEvent::PolicyControl { ... }` variant mapped to a new `JournalEvent::PolicyControlEvent` in vb-s4hfc.

### Recovery Re-Reads Old Decisions, Does Not Re-Run Authorization

- Recovery reads old `DecisionRecord` and durable receipts from the journal.
- Recovery never re-runs `policy.evaluate()` into old semantic decisions.
- Reconciliation of existing receipts stays separate from new effect dispatch.
- Replayed historical receipts remain valid; the replay theorem reconstructs outbox intents under the old pinned context.

### Existing Protections Preserved

- RegisterGrant: opaque checked value with `grant.validate(ticket)` inside adapter. Lifecycle remains vb-hk1j8.
- ExecutorCatalog: checked catalog loaded at startup, `catalog.verify_artifact(&artifact, policy)` in `registered_context()`.
- Pending ticket: canonical `lookup_pending_action_ticket(run, step)`.
- Input: actual I64 slot value from `state.frame.read_slot(input)`.
- Scope: exact artifact/pack/resource from the admitted `AcceptedArtifact`.
- Timeout: grant's `timeout_ms` enforced in `adapter.execute()`.
- SyncAll: `adapter.execute()` calls `self.db.persist(PersistMode::SyncAll)`.

## First Implementation Slice

### Required Changes

**File: `crates/vb_runtime/src/runtime.rs`**
- Add `live_policy: LivePolicy` field to `Runtime` struct.
- Initialize to `LivePolicy::new()` in `Runtime::new()`.
- Add accessor methods: `live_policy()` and `live_policy_mut()`.

**File: `crates/vb_runtime/src/runtime/registered_executor.rs`**
- Modify `execute_registered_action()` to insert policy evaluation:

```rust
pub fn execute_registered_action(
    &mut self,
    adapter: &mut FjallRegister,
    run: RunId,
    wire_step: u64,
    grant: &RegisterGrant,
) -> Result<RegisterReceipt, RegisteredExecutionError> {
    let (ticket, context) = self.registered_context(run, wire_step, grant)?;
    if context.mode != crate::shard::types::RegisteredExecutionMode::Live {
        return Err(ExecutorError::Unauthorized.into());
    }

    // NEW: Evaluate live policy before effect dispatch
    let pinned = match context.artifact.policy_digest {
        _ => PinnedAdmissionPolicy::Artifact(
            PolicyDigest::from_bytes(context.artifact.policy_digest.as_bytes())
        ),
    };
    let record = self.live_policy.evaluate(
        ticket,
        pinned,
        &postcard::to_allocvec(&context.value).map_err(|_| ExecutorError::Internal)?,
        vb_core::value::Taint::Clean,
    )?;

    // NEW: Journal the decision before the effect
    self.journal.append_sequenced(
        RuntimeJournalEvent::PolicyDecision { run, record: record.clone() },
        EventSeq::new(0),  // shard-owned seq via context
    )?;

    if record.outcome != DecisionCode::Allowed {
        return Err(RegisteredExecutionError::PolicyDenied { record });
    }

    // Existing: perform the effect
    let receipt = adapter.execute(grant, ticket, context.value)?;
    self.enqueue_registered_completion(ticket, context.output, receipt.clone())?;
    Ok(receipt)
}
```

- Add `PolicyDenied` variant to `RegisteredExecutionError`.

**File: `crates/vb_runtime/src/journal/chunk_001.rs`**
- Add `PolicyDecision { run: RunId, record: DecisionRecord }` variant to `RuntimeJournalEvent`.

**File: `crates/vb_runtime/src/journal/chunk_002.rs`**
- Map `RuntimeJournalEvent::PolicyDecision` to `JournalEvent::PolicyDecisionEvent` in `convert_to_storage_event()`.

**File: `crates/vb_storage/src/events/event.rs`** (via vb-s4hfc merge)
- Already has `PolicyDecisionEvent { run, seq, decision }` variant. No changes required after vb-s4hfc merge.

### Public Behavior Tests

Tests go in `crates/workspace_tests/tests/` with dedicated `vb_hokkd_live_policy.rs`.

1. **admitted_then_denied**: Configure LivePolicy with `PolicyRule { action: test_action, code: DecisionCode::Denied }`. Submit run, drive to Do node, call `execute_registered_action()`. Expect `Err(RegisteredExecutionError::PolicyDenied { .. })`. No adapter.execute call. Journal contains exactly one `PolicyDecisionEvent` with `outcome: Denied`.

2. **default_deny_all**: Create Runtime with no LivePolicy configuration. Submit run, drive to Do node, call `execute_registered_action()`. Expect `Err(RegisteredExecutionError::PolicyDenied { .. })`. No adapter.execute call.

3. **allowed_effect_exactly_once**: Configure LivePolicy with `PolicyRule { action: test_action, code: DecisionCode::Allowed }`. Submit run, drive to Do node, call `execute_registered_action()`. Expect `Ok(receipt)`. Journal contains exactly one `PolicyDecisionEvent` with `outcome: Allowed` followed by effect evidence.

4. **deny_after_scheduling**: Configure LivePolicy with `Allowed`. Call `execute_registered_action()` → succeeds. Configure LivePolicy with `Denied`. Retry same action ticket. Expect `Err(PolicyDenied)`. Deny after scheduling blocks the effect.

5. **journal_failure_zero_effects**: Patch journal to return error on append. Configure LivePolicy with `Allowed`. Call `execute_registered_action()`. Expect journal error, no adapter.execute call.

6. **retry_reauthorizes**: Configure LivePolicy with `Denied`. Call → fails. Configure with `Allowed`. Call → succeeds. Every retry re-evaluates current policy.

7. **restart_monotonic_deny**: Configure LivePolicy with `Allowed`. Call → succeeds. Call `activate_emergency_deny()`. Call again → fails. Create new Runtime instance → `emergency_active()` is false. Restart resets in-memory deny (correct per master77.11).

8. **replay_reconcile_zero_new_effects**: Write journal with historical `PolicyDecisionEvent` and `ActionCompletedEnvelope`. Call `reconcile_registered_action()`. Expect receipt read from journal, no new `PolicyDecisionEvent`, no adapter.execute call.

### Required Follow-Up (Not First Slice)

1. `RuntimeJournalEvent::PolicyControl` variant for emergency deny deactivation.
2. `RuntimeJournalEvent::PolicyConfigured` variant for policy change audit trail.
3. Integration with actual `vb-h7rex` admitted policy pin after it merges.

## Real Existing Signatures

- `crates/vb_runtime/src/policy.rs`: `LivePolicy::new()`, `configure(digest, rule)`, `activate_emergency_deny()`, `evaluate(ticket, pinned, input_bytes, taint) -> RuntimeResult<DecisionRecord>`
- `crates/vb_runtime/src/runtime.rs:189`: `Runtime { shards, shard_count, journal, policy: vb_core::policy::RuntimePolicy }`
- `crates/vb_runtime/src/runtime/registered_executor.rs:26`: `execute_registered_action(&mut self, adapter: &mut FjallRegister, run: RunId, wire_step: u64, grant: &RegisterGrant) -> Result<RegisterReceipt, RegisteredExecutionError>`
- `crates/vb_storage/src/executor/register.rs:53`: `adapter.execute(grant, ticket, value)`
- `crates/vb_core/src/policy.rs:84`: `PolicyRule { action: ActionId, code: DecisionCode }`
- `crates/vb_core/src/policy.rs:19`: `PolicyDigest` — `from_bytes([u8;32])`, `as_bytes()`

No fabricated helper exists in the current source tree. The `RuntimeJournalEvent::PolicyDecision` variant and `Runtime.live_policy` field are genuinely missing seams.
