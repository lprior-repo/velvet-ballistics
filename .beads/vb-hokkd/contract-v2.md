# Policy Dispatch and Control Seams (vb-hokkd) — Contract v2

Source-grounded Rust contract connecting the accepted pure `LivePolicy` foundation to the actual live effect boundary, with durable policy decisions and monotonic emergency deny.

## Current State (as of this contract)

The runtime has `LivePolicy` and `DecisionRecord` types but **no runtime enforcement**:

- `crates/vb_runtime/src/runtime/registered_executor.rs:execute_registered_action()` calls `registered_context()` → verifies `RegisteredExecutionMode::Live` → `adapter.execute()`. **No policy evaluation between context verification and effect dispatch.**
- `reconcile_registered_action()` calls `adapter.reconcile()` and remains effect-free. Correct.
- `crates/vb_runtime/src/policy.rs:LivePolicy` is a plain owned evaluator with `configure()`, `activate_emergency_deny()`, `evaluate()`. **Not wired into any runtime struct.**
- `crates/vb_runtime/src/runtime.rs:Runtime` has `policy: vb_core::policy::RuntimePolicy` (durability enum: Strict/Journaled/Relaxed) but **no `LivePolicy` instance.**
- `crates/vb_runtime/src/journal/chunk_001.rs:RuntimeJournalEvent` enum has **no policy decision variant.**
- `crates/vb_storage/src/events/event.rs:JournalEvent` enum has **no `PolicyDecisionEvent` variant.**
- `crates/vb_runtime/src/shard/lifecycle/registered_completion.rs:registered_action_context()` returns `RegisteredActionContext` with the actual admitted `AcceptedArtifact` — the source for the admitted policy pin. This method reads the actual input slot value and taint, and currently validates `Taint::Clean` (line 40-41).

## Master §77.3 Binding

Deterministic state transition and live-effect dispatch are separate worlds:

- `transition(A, S, E, B, D) -> (S', J, O, R)` — outbox intents, not live effects.
- `dispatch(O, L) -> effect attempt | denial | suspension` — live policy evaluated here, immediately before `adapter.execute()`.
- Replay reconstructs outbox intents under old policy; new live effects require current policy and grants.
- Authorization occurs at new EFFECT dispatch, not at scheduling.

## Contract

### LivePolicy is Actor-Owned Per-Runtime, Configured via Typed Operations

```rust
impl Runtime {
    /// Runtime owns a single LivePolicy instance. Plain owned, no Arc/Mutex.
    /// LivePolicy::evaluate() takes no global state or ambient time/network/config reads.
    pub fn live_policy(&self) -> &LivePolicy;

    /// Configure live policy with a verified artifact digest and rule.
    /// Durable journal event required before acknowledgement.
    pub fn configure_live_policy(&mut self, digest: PolicyDigest, rule: PolicyRule);

    /// Activate monotonic emergency deny overlay.
    /// Durable journal event required before acknowledgement.
    pub fn activate_emergency_deny(&mut self);
}
```

No `live_policy_mut()` accessor: mutable exposure would bypass transactional control. Each configuration change is a typed operation that journals and acknowledges.

Current default: `LivePolicy::Unconfigured` → all actions denied.

### Per-Run Admission Pin: Retained at Admission, Recovered at Dispatch

The per-run pin is the admitted artifact's policy digest, retained on the admitted run state and recovered for each effect dispatch. It is **not reconstructed** from the current global policy at dispatch time.

```rust
// At admission (existing: RunState already retains admission)
let admission = RunAdmission { artifact_digest, run_id, caps, policy, ... };
// artifact.policy_digest is the per-run pin

// At dispatch (NEW: read from admitted run state)
let run_state = self.shard_for_run(run)?.run_state_get(run)?;
let admission = run_state.admission.as_ref().ok_or(...)?;
let artifact = self.artifact_store
    .load_accepted_artifact(admission.artifact_digest())?;
let pinned = PinnedAdmissionPolicy::Artifact(
    PolicyDigest::from_bytes(artifact.policy_digest.as_bytes())
);
```

`AcceptedArtifact.policy_digest` (line 211 of `crates/vb_storage/src/admission.rs`) is the canonical admitted policy pin. It is computed from the workflow's resource contract at admission via `compute_policy_digest()` (line 230) and persisted with the accepted artifact.

### Policy Evaluation at Effect Dispatch Boundary

**Modified execution path in `execute_registered_action()`:**

```text
execute_registered_action(adapter, run, step, grant)
    → registered_context()          [existing: ticket, artifact, value, output, mode]
    → policy.evaluate(ticket, admission_pin, input_bytes, taint)  [NEW]
    → shard.append_journal_event(PolicyDecision{record})           [NEW]
    → if outcome != Allowed: return Err(PolicyDenied{record})      [NEW]
    → adapter.execute(grant, ticket, value)                       [existing]
    → enqueue_registered_completion(...)                           [existing]
```

Reconcile remains effect-free (never re-evaluates policy, never re-authorizes):

```text
reconcile_registered_action(adapter, run, step, grant)
    → registered_context()
    → adapter.reconcile(grant, ticket, value)  [read-only receipt sync]
    → enqueue_registered_completion(...)
```

### Input Encoding and Taint: Read Actual Slot, Do Not Serialize Unrelated Value

```rust
let record = self.live_policy.evaluate(
    ticket,
    pinned,
    &context.output.encoded,        // actual postcard bytes of slot value
    context.output.taint,           // actual slot taint (currently Clean per registered_action_context)
)?;
```

Source of input bytes: `RegisteredActionContext.output` is an `ActionOutputReady` built in `registered_action_context()` (line 55-63 of `registered_completion.rs`) from the actual I64 slot value read via `state.frame.read_slot(input)`. The encoded bytes and taint are derived from this actual slot value — not a separate `context.value` integer re-serialized.

### Durable DecisionRecord Before Effect

**Ordering invariant:** valid current decision → successful durable journal append → new effect. Journal error or denial/suspension → zero `adapter.execute` calls.

```rust
let record = self.live_policy.evaluate(ticket, pinned, &input_bytes, taint)?;
self.journal_event_for(run)?
    .append_sequenced(RuntimeJournalEvent::PolicyDecision {
        run,
        record: record.clone(),
    })?;
if record.outcome != DecisionCode::Allowed {
    return Err(RegisteredExecutionError::PolicyDenied { record });
}
let receipt = adapter.execute(grant, ticket, context.value)?;
```

The DecisionRecord is the full record with all 11 fields (ticket, admission, current_live, input_digest, taint, outcome, layer, emergency_influence, compat_influence). No fabrication of zero seqs or synthetic digests.

Journal sequence allocation: use the shard's existing `append_journal_event()` infrastructure (line 195 of `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`) which manages per-run `journal_sequences` state and advances the sequence atomically with the append. Do not fabricate `EventSeq::new(0)`.

### Emergency Deny: Durable, Monotonic, Runtime-Scope

Emergency deny is a monotonic overlay evaluated at effect dispatch. Once activated, it remains active for the remainder of the runtime instance.

**Durability requirement:** activation must durably commit to the journal BEFORE acknowledgement. Process restart must re-derive the emergency deny state from the durable control store. This is NOT an in-memory-only transient flag.

**Storage seam (NEW required bounded typed storage):** The production source has no `RuntimeJournal` control namespace currently. Propose new bounded storage:

```rust
// New: crates/vb_storage/src/events/control.rs
pub struct RuntimeControlRecord {
    pub emergency_deny_active: bool,
    pub config_digest: PolicyDigest,  // current live policy config digest
    pub updated_at_seq: EventSeq,
}
```

Storage location: `vb_storage` control namespace separate from run-scoped events. This is a NEW storage seam, not an existing feature. Schema is bounded: single `RuntimeControlRecord` with emergency flag and current policy digest.

**Required API additions to `RuntimeJournal`:**
```rust
fn persist_runtime_control(&self, record: RuntimeControlRecord) -> RuntimeResult<()>;
fn load_runtime_control(&self) -> RuntimeResult<Option<RuntimeControlRecord>>;
```

**Initialization behavior:** On `Runtime::new()`, `load_runtime_control()` to restore emergency deny state and live policy config. If load/corruption/storage failure → fail closed (all actions denied until control record is verified).

**No clearing API in first slice:** Deactivation is a follow-up bead. Emergency deny is monotonic upward; a new activation simply re-affirms it.

**Scope:** Runtime instance (single `Runtime` object). Not per-tenant, not global across runtimes. Each runtime instance manages its own emergency deny state in its control store.

### Recovery Re-Reads Old Decisions, Does Not Re-Run Authorization

- Recovery reads old `DecisionRecord` and durable receipts from the journal.
- Recovery never re-runs `policy.evaluate()` into old semantic decisions.
- Historical receipts remain valid; the replay theorem reconstructs outbox intents under the old pinned context.
- Historical reconciliation under current emergency deny: reconcile reads existing receipts, does NOT create new effects. A missing receipt cannot become a live effect under historical permission.

### Existing Protections Preserved

- `RegisterGrant`: opaque checked value with `grant.validate(ticket)` inside `adapter.execute()` (line 60 of `crates/vb_storage/src/executor/register.rs`).
- `ExecutorCatalog`: checked catalog loaded at startup, `catalog.verify_artifact()` in `registered_context()`.
- Pending ticket: canonical `lookup_pending_action_ticket(run, wire_step)`.
- Input: actual I64 slot value from `state.frame.read_slot(input)`.
- Scope: exact artifact/pack/resource from the admitted `AcceptedArtifact`.
- Timeout: grant's `timeout_ms` enforced in `adapter.execute()`.
- SyncAll: `adapter.execute()` calls `self.db.persist(PersistMode::SyncAll)` (line 67).
- Receipt idempotency: `adapter.reconcile()` returns existing matching receipt without writing (line 79-85).

## Required Changes

### New: Durable Policy Decision Journal Event

**File: `crates/vb_runtime/src/journal/chunk_001.rs`**
Add variant to `RuntimeJournalEvent`:
```rust
/// Live policy decision evaluated before effect dispatch.
PolicyDecision {
    /// Run identifier.
    run: RunId,
    /// Full decision record with all identity fields.
    record: DecisionRecord,
},
```

### New: Storage-Side Policy Decision Event

**File: `crates/vb_storage/src/events/event.rs`**
Add variant to `JournalEvent`:
```rust
/// Durable live policy decision evaluated before effect dispatch.
PolicyDecisionEvent {
    run: RunId,
    seq: EventSeq,
    decision: DecisionRecord,
},
```

Wire this through:
- `crates/vb_storage/src/events/wire_serialize.rs` — serialization
- `crates/vb_storage/src/events/wire_deserialize.rs` — deserialization
- `crates/vb_storage/src/codec/kind_parity.rs` — parity check
- `crates/vb_runtime/src/journal/chunk_002.rs:storage_event()` — conversion

### New: Runtime Control Store for Emergency Deny Durability

**New file: `crates/vb_storage/src/events/control.rs`**
Define `RuntimeControlRecord` struct and control namespace accessors.

**File: `crates/vb_storage/src/journal/mod.rs`**
Add control record storage methods to `FjallJournal`.

**File: `crates/vb_runtime/src/journal/chunk_002.rs`**
Implement `persist_runtime_control()` and `load_runtime_control()` on `StorageRuntimeJournal`.

### Modified: Runtime Owns LivePolicy Instance

**File: `crates/vb_runtime/src/runtime.rs`**
- Add `live_policy: LivePolicy` field to `Runtime` struct (line 189).
- Initialize to `LivePolicy::new()` in `Runtime::new()` (line 218).
- Add typed operations: `live_policy()`, `configure_live_policy()`, `activate_emergency_deny()`.
- On construction, `load_runtime_control()` to restore state. If load fails → fail closed (log, continue but all actions denied).

### Modified: Insert Policy Evaluation at Dispatch

**File: `crates/vb_runtime/src/runtime/registered_executor.rs`**
- Modify `execute_registered_action()` (line 26) to:
  1. Call `self.live_policy.evaluate(ticket, pinned, input_bytes, taint)` before `adapter.execute()`
  2. Journal the decision via `RuntimeJournalEvent::PolicyDecision`
  3. Check outcome and return `PolicyDenied` if not Allowed
- Add `PolicyDenied { record: DecisionRecord }` variant to `RegisteredExecutionError` enum (line 8)

**Do NOT modify `reconcile_registered_action()`.** It remains purely read-only receipt reconciliation without policy re-evaluation.

### New: Input Byte/Taint Extraction

**File: `crates/vb_runtime/src/runtime/registered_executor.rs`**
Add helper to extract policy evaluation inputs from the registered context:
```rust
fn policy_evaluation_inputs(context: &RegisteredActionContext) -> (&[u8], Taint) {
    (&context.output.encoded, context.output.taint)
}
```

## Public API Tests

Tests go in `crates/workspace_tests/tests/vb_hokkd_live_policy.rs`.

1. **admitted_then_denied**: Configure `LivePolicy` with `PolicyRule { action: test_action, code: DecisionCode::Denied }`. Submit run, drive to Do node, call `execute_registered_action()`. Expect `Err(PolicyDenied { record })`. No `adapter.execute` call. Journal contains exactly one `PolicyDecisionEvent` with `outcome: Denied`.

2. **default_deny_all**: Create `Runtime` with no `LivePolicy` configuration. Submit run, drive to Do node, call `execute_registered_action()`. Expect `Err(PolicyDenied { .. })`. No `adapter.execute` call.

3. **allowed_effect_exactly_once**: Configure `LivePolicy` with `PolicyRule { action: test_action, code: DecisionCode::Allowed }`. Submit run, drive to Do node, call `execute_registered_action()`. Expect `Ok(receipt)`. Journal contains exactly one `PolicyDecisionEvent` with `outcome: Allowed` followed by effect evidence.

4. **deny_after_scheduling_before_first_execution**: Configure `LivePolicy` with `Allowed`. Drive run to Do node (schedules action, creates pending ticket). Call `runtime.activate_emergency_deny()` → durable emergency deny commit. Call `execute_registered_action()`. Expect `Err(PolicyDenied { record })` with `layer: Emergency`. No effect produced. **Key: deny occurs AFTER scheduling but BEFORE first execution — the effect never fires even though the ticket was already created.**

5. **journal_failure_zero_effects**: Patch journal to return error on `append_sequenced` for `PolicyDecision` events. Configure `LivePolicy` with `Allowed`. Call `execute_registered_action()`. Expect journal error propagated, no `adapter.execute` call.

6. **retry_reauthorizes**: Configure `LivePolicy` with `Denied`. Call → fails. Configure with `Allowed`. Call → succeeds. Every retry re-evaluates current policy.

7. **restart_preserves_emergency_deny**: Configure `LivePolicy` with `Allowed`. Call → succeeds. Call `activate_emergency_deny()` → durable commit. Create new `Runtime` instance loading from same control store. Call `execute_registered_action()` → fails. **Key: emergency deny survives restart because it's durable, not in-memory only.**

8. **corrupted_control_store_fails_closed**: Corrupt the control store file. Create `Runtime` instance. Call `execute_registered_action()` → fails (fail-closed). Runtime does not panic or silently continue.

9. **replay_reconcile_zero_new_effects**: Write journal with historical `PolicyDecisionEvent` and `ActionCompletedEnvelope`. Call `reconcile_registered_action()`. Expect receipt read from journal, no new `PolicyDecisionEvent`, no `adapter.execute` call.

10. **historical_reconcile_under_current_deny**: Configure `LivePolicy` with `Denied`. Write historical journal with prior `PolicyDecisionEvent(Allowed)` and `ActionCompletedEnvelope`. Call `reconcile_registered_action()`. Expect receipt read, no new effect. Current deny does not invalidate past receipts; missing receipt remains missing (no synthetic effect created).

## Accepted vs Pending Dependencies

### Accepted (in production)
- `LivePolicy` evaluator in `crates/vb_runtime/src/policy.rs`
- `DecisionRecord` in `crates/vb_core/src/policy.rs`
- `RuntimeJournalEvent` enum in `crates/vb_runtime/src/journal/chunk_001.rs`
- `JournalEvent` enum in `crates/vb_storage/src/events/event.rs`
- `execute_registered_action()` in `crates/vb_runtime/src/runtime/registered_executor.rs`
- `reconcile_registered_action()` in same file
- `RegisteredActionContext` in `crates/vb_runtime/src/shard/lifecycle/registered_completion.rs`
- `AcceptedArtifact.policy_digest` field in `crates/vb_storage/src/admission.rs`
- Shard journal sequence management in `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`

### Pending (required by this contract)
- `RuntimeJournalEvent::PolicyDecision` variant — NEW
- `JournalEvent::PolicyDecisionEvent` variant — NEW
- `RuntimeControlRecord` and control store — NEW
- `Runtime.live_policy` field and typed operations — NEW
- `RegisteredExecutionError::PolicyDenied` variant — NEW

### Not Part of This Slice
- `RuntimeJournalEvent::PolicyConfigured` variant for policy change audit trail — follow-up bead
- Emergency deny deactivation API — follow-up bead
- Full grant lifecycle expiry/revocation — vb-hk1j8
- Per-tenant or global emergency deny scope — future bead, scope is runtime-instance only

## Unresolved Source Questions

1. **Policy config durability**: The contract requires `configure_live_policy()` to journal the configuration. The existing `LivePolicy.configure()` method is an in-memory state change with no journaling. The Runtime-level `configure_live_policy()` must journal a `PolicyConfigured` event before updating the evaluator. **Status: requires new `RuntimeJournalEvent::PolicyConfigured` variant — labeled as follow-up but technically required for complete durability.**

2. **DecisionRecord serialization in JournalEvent**: `DecisionRecord` has `Option<SourceSpanId>` and other fields. The `JournalEvent::PolicyDecisionEvent` wire format must accommodate the full record. **Status: vb-s4hfc child provides accepted stable wire format for `DecisionRecord` — dependency is accepted.**

3. **Control store atomicity**: `persist_runtime_control()` must be atomic with `load_runtime_control()`. If control store write is torn (power failure mid-write), the reader must see either old or new record, not garbage. **Status: requires Fjall-level durability guarantees — presumed from existing `PersistMode::SyncAll` semantics but not explicitly proven.**

4. **Input bytes source for non-I64 slot values**: `registered_action_context()` currently only handles `SlotValue::I64`. If a workflow uses a different slot value type for its Do node input, `registered_action_context()` fails with `InvalidActionCompletion`. This is a pre-existing limitation, not introduced by this contract. **Status: pre-existing, out of scope.**

## Candidate Contract Status

This is a **candidate** contract, not yet accepted. It addresses the specific corrections:
- Durable monotonic emergency deny (not in-memory only)
- Per-run admission pin retained from admitted artifact (not reconstructed)
- Actual slot value/taint for policy evaluation (not hardcoded)
- Shard-owned journal sequence allocation (not fabricated)
- No `live_policy_mut()` — typed operations only
- Historical reconciliation preserves old decisions without new effects
- Existing protections (grant validate, catalog verify, SyncAll, idempotent receipts) preserved

Awaiting acceptance before implementation.