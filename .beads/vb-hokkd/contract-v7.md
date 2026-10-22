# vb-hokkd: Runtime live policy evaluation with durable decision journal

Scope: Evaluate live policy at execute time, persist decision record before effect, durable emergency deny overlay.

## Source state (frozen 6e958d14)

**Runtime policy field:** `crates/vb_runtime/src/runtime.rs:189-194` - owns `vb_core::policy::RuntimePolicy` (durability only). `LivePolicy` evaluator exists in `crates/vb_runtime/src/policy.rs` but not wired in.

**LivePolicy evaluator:** `policy.rs:91-105` - `evaluate(ticket, pinned_admission, input_bytes, taint) -> DecisionRecord`. Pure, read-only, takes admission per-call. Does NOT reject `PinnedAdmissionPolicy::Unverified`. In-memory `emergency_deny_active` only.

**Execute signature:** `runtime/registered_executor.rs:26-34` - `execute_registered_action(&mut self, adapter, run, wire_step, grant) -> Result<RegisterReceipt, RegisteredExecutionError>`. Calls `adapter.execute(grant, ticket, context.value)` after mode check.

**Reconcile signature:** `runtime/registered_executor.rs:44-54` - reads receipt via `adapter.reconcile()`, never executes.

**Registered context:** `shard/lifecycle/registered_completion.rs:7-12` - `{ artifact, output, value, mode }`. Missing: policy pin, input bytes, taint. Builder at `:14-63` reads admission, loads artifact, reads slot value.

**RuntimeJournalEvent:** `journal/chunk_001.rs:16-204` - 24 variants, no policy decision variant.

**Storage PolicyDecisionEvent:** `crates/vb_storage/src/events/event.rs:315-322` - exists with `decision: DecisionRecord`.

**RuntimeJournal trait:** `journal/chunk_001.rs:236-316` - `append()`, `storage_journal()`, `append_sequenced()`, `convert_to_storage_event()`.

**Shard journal ops:** `shard/impl_parts/chunk_001.rs:193-232,271-316` - `append_journal_event()`, `append_journal_event_batch()`, `journal_sequence_for()`, `advance_journal_sequence()`.

**Durable emergency deny:** `crates/vb_storage/src/journal/emergency_deny.rs` - `emergency_deny_state()` / `activate_emergency_deny()` on `FjallJournal`. Fixed key `active`, one-byte versioned marker.

**Strict append:** `crates/vb_storage/src/journal/append.rs:39-68` - `append_strict()` with `PersistMode::SyncAll`.

**Writer durable flush:** `crates/vb_storage/src/queue/writer/flush.rs:21-69` - `flush_batch_durable()` does SyncAll without closing queue.

## Proposed changes

**1. Runtime owns LivePolicy:** Replace `policy: vb_core::policy::RuntimePolicy` with `live_policy: LivePolicy`. Keep durability policy separate.

**2. Runtime configure API (NEW):**
```rust
pub fn configure_live_policy(&mut self, digest: PolicyDigest, rule: PolicyRule);
pub fn activate_emergency_deny(&mut self);
pub fn live_policy(&self) -> &LivePolicy;
```

**3. Extended context (NEW fields):**
```rust
pub(crate) struct RegisteredActionContext {
    artifact, output, value, mode,  // existing
    policy_pin: PinnedAdmissionPolicy,
    input_bytes: Vec<u8>,
    taint: Taint,
}
```
Builder extracts policy pin from `state.admission`, captures input bytes (`postcard::to_allocvec` of slot value) and taint from `state.frame`.

**4. Execute evaluates policy (NEW):**
- Read durable emergency deny via `self.journal.storage_journal()?.emergency_deny_state()`. Error = fail closed. If Active, activate in-memory `live_policy`.
- Evaluate: `let record = self.live_policy.evaluate(ticket, context.policy_pin, &context.input_bytes, context.taint)?;`
- Explicit Unverified denial: if `context.policy_pin == Unverified`, set `record.outcome = Denied`, `record.layer = Admission`.
- Persist BEFORE execute: call new `self.journal.append_policy_decision_strict(record)?`.
- If not Allowed, return `PolicyDenied(record)` error, never call `adapter.execute()`.

**5. Reconcile unchanged:** Always reads existing receipt. No policy eval. No execute.

**6. New RuntimeJournalEvent variant (NEW):**
```rust
PolicyDecision { run: RunId, decision: vb_core::policy::DecisionRecord }
```

**7. New RuntimeJournal trait method (NEW):**
```rust
fn append_policy_decision_strict(&self, record: DecisionRecord) -> RuntimeResult<()>
    default: Err(UnsupportedOperation { operation: "strict_policy_decision_append" });
```

**8. StorageRuntimeJournal impl (NEW):** Converts to `vb_storage::JournalEvent::PolicyDecisionEvent`, calls `append_strict()`.

**9. QueuedStorageRuntimeJournal impl (NEW):** Enqueues event with strict tag, waits for `flush_batch_durable()` ack.

**10. New error (NEW):** `PolicyDenied(DecisionRecord)` variant on `RuntimeError`.

## Test scenarios

1. Configure Allow after scheduling: schedule with Unconfigured policy, configure Allow, execute succeeds.
2. Allow persisted before effect: verify `PolicyDecisionEvent(Allowed)` in journal before adapter side effect observed.
3. Deny produces zero effect: configure Deny, execute returns `PolicyDenied`, no side effect, pending tickets unchanged.
4. Unverified admission rejected: `PinnedAdmissionPolicy::Unverified` → Denied at Admission layer.
5. Durable emergency deny after reopen: activate, flush, drop, reopen, execute still Denied.
6. Corrupt marker fail closed: malformed marker → error, not Inactive.
7. Byte+taint digest: different taint → different `input_digest`.
8. Reconcile under deny: allow→deny change doesn't affect existing receipt reconciliation.
9. Persist fault blocks execute: `append_strict` failure prevents execute call and side effect.

## Out of scope

Grant expiry/revocation, full policy language, cross-action policy, admission pin verification, reconcile path changes.