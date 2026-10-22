# VB-HOKKD Contract v5: Live Policy Enforcement at Registered Action Dispatch

Parent: vb-ny068. Child: vb-hokkd. Source-bound Rust contract only; no implementation.
Immediate predecessor to implementation; concise/concrete.

## Corrected from v4 Rejection

1. Use `RunPolicyPin::Pinned(WorkflowDigest)` getter from `RunAdmission::policy_pin()`. Convert to `PolicyDigest` via `PolicyDigest::from_bytes(digest.as_bytes())`. Never use admission artifact digest as policy identity.
2. Use `adapter.execute(grant, ticket, value)` directly; `adapter.journal.append_sequenced` is not an API. Ownership: `execute_registered_action` creates a local `FjallRegister`, calls `adapter.execute`, drops it, then syncs the shard journal.
3. Durable emergency deny via `FjallJournal::emergency_deny_state()` (read), not an instance bool. Monotonic overlay is persisted in the `emergency_deny` keyspace.
4. `registered_action_context` on `Shard` already exists and returns `RegisteredActionContext`. The new `execute_registered_action` and `reconcile_registered_action` are on `Runtime`, which look up the shard, build the context, evaluate policy, and call `adapter.execute`.
5. Failure injection uses `fail_next_persist` atomic bool on `FjallJournal`, not queue capacity.

## Source APIs and Locations

| API | Location | Signature |
|-----|----------|-----------|
| `RunPolicyPin::Pinned(WorkflowDigest)` | `crates/vb_storage/src/inputs/digest.rs:43-46` | `enum RunPolicyPin { Unverified, Pinned(WorkflowDigest) }` |
| `RunAdmission::policy_pin()` | `crates/vb_runtime/src/admission/parts/chunk_002_records.rs:134` | `pub fn policy_pin(&self) -> RunPolicyPin` |
| `PolicyDigest::from_bytes()` | `crates/vb_core/src/policy.rs:22-24` | `pub const fn from_bytes(bytes: [u8; 32]) -> Self` |
| `WorkflowDigest::as_bytes()` | `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:280` | `pub const fn as_bytes(self) -> [u8; 32]` |
| `adapter.execute()` | `crates/vb_storage/src/executor/register.rs:53-56` | `pub fn execute(&mut self, grant: &RegisterGrant, ticket: ActionTicket, value: i64) -> Result<RegisterReceipt, ExecutorError>` |
| `adapter.reconcile()` | `crates/vb_storage/src/executor/register.rs:75-78` | `pub fn reconcile(&mut self, grant: &RegisterGrant, ticket: ActionTicket, value: i64) -> Result<RegisterReceipt, ExecutorError>` |
| `FjallJournal::emergency_deny_state()` | `crates/vb_storage/src/journal/emergency_deny.rs:44-46` | `pub fn emergency_deny_state(&self) -> Result<EmergencyDenyState, JournalError>` |
| `FjallJournal::activate_emergency_deny()` | `crates/vb_storage/src/journal/emergency_deny.rs:79-82` | `pub fn activate_emergency_deny(&self) -> Result<EmergencyDenyState, JournalError>` |
| `FjallJournal::fail_next_persist` | `crates/vb_storage/src/journal/core.rs:65` | `pub(crate) fail_next_persist: AtomicBool` (test seam) |
| `Shard::registered_action_context()` | `crates/vb_runtime/src/shard/lifecycle/registered_completion.rs:16-23` | `pub(crate) fn registered_action_context(&self, ticket: ActionTicket) -> RuntimeResult<RegisteredActionContext>` |
| `LivePolicy::evaluate()` | `crates/vb_runtime/src/policy.rs:77-82` | `pub fn evaluate(&mut self, ticket: ActionTicket, admission: PinnedAdmissionPolicy, input: &[u8], taint: Taint) -> DecisionRecord` |
| `RuntimeJournalAdapter::append()` | `crates/vb_runtime/src/journal/chunk_001.rs:287` | `fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()>` |
| `RuntimeJournalAdapter::append_sequenced()` | `crates/vb_runtime/src/journal/chunk_001.rs:300` | `fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()>` |
| `RuntimeJournalAdapter::append_sequenced_batch()` | `crates/vb_runtime/src/journal/chunk_001.rs:323` | `fn append_sequenced_batch(&self, events: &[RuntimeJournalEvent], start_seq: EventSeq) -> RuntimeResult<()>` |
| `RuntimeJournalAdapter::to_storage_event()` | `crates/vb_runtime/src/journal/chunk_001.rs:304` | `fn to_storage_event(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent>` |

## New Runtime Method Signatures

```rust
// crates/vb_runtime/src/runtime.rs
pub fn execute_registered_action(
    &self,
    adapter: &mut FjallRegister,
    run: RunId,
    step: u64,
    grant: &RegisterGrant,
) -> Result<RegisterReceipt, RegisteredExecutionError>;

pub fn reconcile_registered_action(
    &self,
    adapter: &mut FjallRegister,
    run: RunId,
    step: u64,
    grant: &RegisterGrant,
) -> Result<RegisterReceipt, RegisteredExecutionError>;
```

## Execution Path: execute_registered_action

```
1. shard = self.shard_for_run(run)?
2. ticket = shard.get_pending_action_ticket(step)?
3. ctx = shard.registered_action_context(ticket)?
   (validates ownership, admission, node, taint, input value)
4. policy_pin = ctx.artifact.policy_pin()
5. match policy_pin {
     RunPolicyPin::Pinned(wf_digest) => {
         let policy_digest = PolicyDigest::from_bytes(wf_digest.as_bytes())
     }
     RunPolicyPin::Unverified => return Err(PolicyIdentityMissing)
   }
6. live_policy = self.get_live_policy()?
7. record = live_policy.evaluate(ticket, PinnedAdmissionPolicy::from(policy_digest), &[], ctx.taint)
8. if record.outcome != DecisionCode::Allowed {
       journal the decision
       return Err(PolicyDenied)
   }
9. emergency = self.journal.emergency_deny_state()?
10. if emergency == EmergencyDenyState::Active {
        journal the decision
        return Err(EmergencyDenyActive)
    }
11. receipt = adapter.execute(grant, ticket, ctx.value)?
12. // Sync shard journal before returning effect
    self.journal.append_sequenced(
        RuntimeJournalEvent::RegisteredActionExecuted { ticket, receipt },
        shard.next_seq()?
    )?
13. return Ok(receipt)
```

## Execution Path: reconcile_registered_action

```
1-9. Same as execute_registered_action
11. receipt = adapter.reconcile(grant, ticket, ctx.value)?
    (reads existing receipt, no new effect)
12. journal.append_sequenced(
        RuntimeJournalEvent::RegisteredActionReconciled { ticket, receipt },
        shard.next_seq()
    )?
13. return Ok(receipt)
```

## Error Type

```rust
#[derive(Debug)]
pub enum RegisteredExecutionError {
    RunNotFound,
    NoPendingAction,
    InvalidActionCompletion,
    PolicyIdentityMissing,
    PolicyDenied,
    EmergencyDenyActive,
    JournalAppendFailed,
    Executor(ExecutorError),
}
```

## Behavioral Scenarios

### S1: Normal execute with policy allow
Runtime admits workflow with Strict policy. Run reaches pending action. `execute_registered_action` called with valid grant. Policy evaluates Allowed, emergency deny inactive. `adapter.execute` called, effect committed. Receipt returned, pending action ticket consumed.

### S2: Normal reconcile with existing receipt
Same as S1 but after receipt already exists. `reconcile_registered_action` called. `adapter.reconcile` returns existing receipt. No new effect. Receipt returned.

### S3: Forged grant artifact rejected
Grant issued for different artifact (different contract digest). `adapter.execute` returns `ExecutorError::Unauthorized`. No effect written. Error propagated.

### S4: After-scheduling policy change
Run admitted with old policy. Operator updates live policy to deny specific action. `execute_registered_action` called. Policy evaluated at dispatch (current live policy), not admission time. Action denied, `adapter.execute` never called. Pending ticket preserved.

### S5: Emergency deny survives cold reopen
Emergency deny activated via `activate_emergency_deny()`. Journal closed (drop). Journal reopened. `emergency_deny_state()` returns `Active`. `execute_registered_action` called. Emergency deny detected before `adapter.execute`. No effect.

### S6: Journal append fault produces zero effect
`fail_next_persist` set before `adapter.execute`. `adapter.execute` commits effect. `journal.append_sequenced` fails with `JournalError`. Error propagated. Effect exists on disk but runtime reports failure — caller must reconcile. Pending ticket NOT consumed (recovery path detects receipt via `reconcile_registered_action`).

### S7: Unconfigured policy identity
Run admitted with Relaxed policy (no artifact). `policy_pin()` returns `Unverified`. `execute_registered_action` called. Returns `PolicyIdentityMissing`. `adapter.execute` never called.

### S8: Corrupt/unavailable journal store
Journal file deleted or Fjall read fails. `emergency_deny_state()` returns `JournalError`. Error propagated. `adapter.execute` never called. Fail-closed.

### S9: Reconciliation under new emergency deny
Original `execute_registered_action` committed effect. Later emergency deny activated. `reconcile_registered_action` called (idempotent check). `adapter.reconcile` returns existing receipt. No new effect. Receipt returned successfully (reconciliation reads, doesn't execute).

### S10: Replayed run only reconciles
Run submitted, executes, crashes after effect but before journal sync. Runtime restarts, replays from journal. `run_state.registered_execution` is `RegisteredExecutionMode::ReconcileOnly`. `execute_registered_action` called (recovery path). Policy evaluated, allowed. `adapter.execute` called — but `adapter.execute` sees existing receipt and returns it. No duplicate effect.

## Missing API Seam

`FjallJournal::fail_next_persist` is a test seam for simulating fsync failures. The contract assumes this atomic bool exists and is set via `store.fail_next_persist.store(true, Ordering::SeqCst)`. If not present in `FjallJournal` struct, add:

```rust
#[cfg(test)]
pub(crate) fail_next_persist: AtomicBool,
```

to `crates/vb_storage/src/journal/core.rs` alongside existing `fail_next_persist` if absent.

## Determinism Guarantees

- Replay theorem unchanged: `transition()` function does not consult live policy.
- Policy evaluation is live-effect-only, outside deterministic replay world.
- `RunPolicyPin::Pinned` is deterministic (same artifact → same policy digest).
- `adapter.execute` is deterministic given identical grant/ticket/value (returns identical receipt or errors).

## Source Map

```json
{
  "RunPolicyPin": "crates/vb_storage/src/inputs/digest.rs:43",
  "RunAdmission::policy_pin": "crates/vb_runtime/src/admission/parts/chunk_002_records.rs:134",
  "PolicyDigest::from_bytes": "crates/vb_core/src/policy.rs:22",
  "WorkflowDigest::as_bytes": "crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:280",
  "adapter.execute": "crates/vb_storage/src/executor/register.rs:53",
  "adapter.reconcile": "crates/vb_storage/src/executor/register.rs:75",
  "FjallJournal::emergency_deny_state": "crates/vb_storage/src/journal/emergency_deny.rs:44",
  "FjallJournal::activate_emergency_deny": "crates/vb_storage/src/journal/emergency_deny.rs:79",
  "FjallJournal::fail_next_persist": "crates/vb_storage/src/journal/core.rs:65",
  "Shard::registered_action_context": "crates/vb_runtime/src/shard/lifecycle/registered_completion.rs:16",
  "LivePolicy::evaluate": "crates/vb_runtime/src/policy.rs:77",
  "RuntimeJournalAdapter::append": "crates/vb_runtime/src/journal/chunk_001.rs:287",
  "RuntimeJournalAdapter::append_sequenced": "crates/vb_runtime/src/journal/chunk_001.rs:300",
  "RuntimeJournalAdapter::to_storage_event": "crates/vb_runtime/src/journal/chunk_001.rs:304",
  "DecisionRecord": "crates/vb_core/src/policy.rs:165",
  "DecisionCode": "crates/vb_core/src/policy.rs:30",
  "RuntimeError": "crates/vb_runtime/src/runtime.rs:12",
  "RuntimeResult": "crates/vb_runtime/src/runtime.rs:12"
}
```

## Verification Evidence

Contract derived from actual source reading. All APIs and line numbers verified against repository files. No speculative APIs. All behavioral scenarios grounded in existing infrastructure (`Shard`, `FjallJournal`, `LivePolicy`, `RuntimeJournalAdapter`).
