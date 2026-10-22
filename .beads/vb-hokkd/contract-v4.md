# Registered Action Dispatch Contract V4

Source-bound to actual signatures in the isolated workspace. Defines the
complete live-effect authorization boundary for registered action execution.

## Current Actual Signatures (verified, unchanged)

```rust
// crates/vb_runtime/src/runtime/registered_executor.rs (SHA256: 42cebc4f)
pub fn execute_registered_action(
    &mut self,
    adapter: &mut FjallRegister,
    run: RunId,
    wire_step: u64,
    grant: &RegisterGrant,
) -> Result<RegisterReceipt, RegisteredExecutionError>

pub fn reconcile_registered_action(
    &mut self,
    adapter: &mut FjallRegister,
    run: RunId,
    wire_step: u64,
    grant: &RegisterGrant,
) -> Result<RegisterReceipt, RegisteredExecutionError>
```

```rust
// crates/vb_storage/src/executor/register.rs (SHA256: 78e07824)
pub fn execute(&mut self, grant: &RegisterGrant, ticket: ActionTicket, value: i64)
    -> Result<RegisterReceipt, ExecutorError>

pub fn reconcile(&mut self, grant: &RegisterGrant, ticket: ActionTicket, value: i64)
    -> Result<RegisterReceipt, ExecutorError>
```

```rust
// crates/vb_runtime/src/policy.rs (SHA256: 2c11370f)
pub fn evaluate(
    &self,
    ticket: ActionTicket,
    pinned_admission: PinnedAdmissionPolicy,
    input_bytes: &[u8],
    taint: Taint,
) -> RuntimeResult<DecisionRecord>
```

```rust
// crates/vb_core/src/policy.rs (SHA256: 35a38e9a)
pub struct DecisionRecord {
    pub ticket: ActionTicket,
    pub admission: PinnedAdmissionPolicy,
    pub current_live: CurrentPolicyIdentity,
    pub input_digest: PolicyDigest,
    pub taint: Taint,
    pub outcome: DecisionCode,
    pub layer: PolicyLayer,
    pub emergency_influence: InfluenceState,
    pub compat_influence: InfluenceState,
}
```

## Proposed Owned Runtime API (not yet implemented)

```rust
pub struct LivePolicy {
    current_live: CurrentLivePolicy,
    emergency_deny_active: bool,
}

impl LivePolicy {
    pub fn new() -> Self { ... }
    pub fn configure(&mut self, digest: PolicyDigest, rule: PolicyRule) { ... }
    pub fn activate_emergency_deny(&mut self) { ... }
    pub fn evaluate(
        &self,
        ticket: ActionTicket,
        pinned_admission: PinnedAdmissionPolicy,
        input_bytes: &[u8],
        taint: Taint,
    ) -> RuntimeResult<DecisionRecord>;
}
```

## Proposed Owned Context Extension (not yet implemented)

```rust
pub(crate) struct RegisteredActionContext {
    pub artifact: vb_storage::AcceptedArtifact,
    pub output: ActionOutputReady,
    pub value: i64,
    pub mode: RegisteredExecutionMode,
    // Proposed additions:
    pub pinned_admission: PinnedAdmissionPolicy,
    pub input_bytes: Vec<u8>,
    pub input_taint: Taint,
}
```

## Dispatch Boundary Before adapter.execute

```text
1. Resolve registered_context(run, wire_step, grant)
2. Validate mode == Live (reconcile path unchanged)
3. Verify catalog + artifact/grant binding (existing)
4. Evaluate: policy.evaluate(ticket, ctx.pinned_admission, ctx.input_bytes, ctx.input_taint)
   -> DecisionRecord with all nine fields populated
5. Journal: adapter.journal.append_sequenced(PolicyDecision { record })
   with SyncAll durability before any effect
6. Match record.outcome:
   - Allowed: proceed to adapter.execute(grant, ticket, value)
   - Denied: return Err(RegisteredExecutionError::PolicyDenied(record))
   - Suspended: return Err(RegisteredExecutionError::PolicySuspended(record))
7. Enqueue completion (existing behavior)
```

## Admission Pin Binding

The pinned admission is retained from `RunState.admission` at run admission time,
not reconstructed at dispatch. This ensures the policy evaluated at live-effect
time is bound to the exact artifact admitted, even if the policy changes later.

```rust
// RunState already has: pub admission: Option<RunAdmission>
// RegisteredActionContext reads it via state.admission
let pinned = match &state.admission {
    Some(admission) => PinnedAdmissionPolicy::Artifact(admission.artifact_digest()),
    None => PinnedAdmissionPolicy::Unverified,
};
```

## Input Identity

The input bytes are the actual I64 value serialized, not fabricated. Taint is
read from the actual frame at dispatch, not assumed. No zero/Clean defaults.

```rust
let input_bytes = postcard::to_allocvec(&SlotValue::I64(value))?;
let input_taint = frame.read_taint(input_slot)?;
```

## Default Deny

A new `Runtime` starts with `LivePolicy::new()`, which has `current_live =
CurrentLivePolicy::Unconfigured`. The first action evaluates to `DecisionCode::Denied`
at the `Live` layer with `InfluenceState::Inactive`. No effect occurs. No silently
restored Allow.

## Emergency Overlay (durable store-scoped marker)

The emergency deny overlay is a durable store-scoped marker, not a runtime-instance
bool. The marker is written and read through the storage journal.

- Marker corruption, read failure, or unavailability → fail closed (deny all effects)
- Monotonic: once activated, cannot be cleared or reset
- Runtime may cache pure policy config, but overlay state is always reloaded from storage
- No `clear_emergency_deny` or `reset` API
- No `()` acknowledgement of failed persistence

## Journal Durability

Every policy decision — both Allowed and Denied — is durably appended to the journal
before any effect is dispatched.

- `append_sequenced` with shard-owned `EventSeq` (no reserved-seq races)
- `SyncAll` durability mode (same as receipt persistence)
- Journal failure → no new effect, return journal error, pending action preserved
- Decision records are neutral facts: they do not alter lifecycle state (per
  `is_lifecycle_affecting` in vb_storage recovery)

## Ownership and Mutability

- `LivePolicy` is owned by `Runtime`; configured via trusted typed setter
- No raw mutable policy field exposed
- No `set_policy`, `set_store`, `reset`, or forged-bool setters
- `&mut self` ownership in `execute_registered_action` preserved
- `&self` in `evaluate` preserved (no side effects)
- No invented locks or unbounded allocations

## Unresolved (not claimed by this contract)

- Full grant revocation and expiry lifecycle
- Per-target capability granularity (current rule is per-action)
- Multi-rule policy evaluation (current rule is single-rule bounded)
- Runtime instance restart recovery of policy state

## Acceptance Scenarios

### 1. Default deny, zero effects
New runtime, no policy configured. Submit run with registered action. Call
`execute_registered_action`. Expect `PolicyDenied` with `layer = Live`,
`influence = Inactive`. `adapter.effect_count()` is 0.

### 2. Allowed decision, exact record before effect
Configure policy to allow the action. Execute. Expect `PolicyDecisionEvent`
in journal with all nine fields populated, `SyncAll` committed, then the
receipt. `adapter.effect_count()` is 1.

### 3. Policy change after scheduling denies new execution
Schedule run. Before execute, reconfigure policy to deny. Execute. Expect
`PolicyDenied`. Journal contains denial record with the new policy digest.
`adapter.effect_count()` is 0.

### 4. Emergency overlay denies future effects, preserves history
Execute one action (allowed). Activate emergency overlay. Schedule second
action. Execute. Expect `PolicyDenied` with `layer = Emergency`.
`adapter.effect_count()` is 1 (only first). Journal contains both the
original allowed decision and the new emergency denial.

### 5. Append failure: zero effects, pending preserved
Fault the journal (e.g., fill queue, trigger `QueueFull`). Execute. Expect
`RuntimeError::StorageJournalAppend`. `adapter.effect_count()` is 0. Run
state still has pending action (not completed, not failed).

### 6. Distinct identities: admission, policy, input
Three separate runs with same action: different artifact digests, different
live policy digests, different input values. Each produces a distinct
`input_digest` and distinct `current_live` in the decision record. No
collision.

### 7. Legacy Unverified cannot mint authority
Run with `PinnedAdmissionPolicy::Unverified` (no artifact admission).
Policy configured to allow. Execute. Expect `PolicyDenied` because
Unverified admission is not a verified artifact pin. No fabricated digest.

### 8. Historical reconcile: no new effect under current denial
Execute action, get receipt. Reconfigure policy to deny. Call
`reconcile_registered_action`. Expect existing receipt returned, no new
effect, `adapter.effect_count()` unchanged. Catalog requires receipt
recovery for the artifact.

## Contract Sources

| File | SHA256 |
|------|--------|
| crates/vb_runtime/src/runtime/registered_executor.rs | 42cebc4f6cbd4b2d5e1d70a0d730fb30064cbd13924630f95da235e6ef7acdfb |
| crates/vb_runtime/src/shard/lifecycle/registered_completion.rs | e2b041e1ce48911842ac8c7388ec76fed3cd9ff57aa1b74f892c87840dd0de5d |
| crates/vb_runtime/src/policy.rs | 2c11370fc1d07ae5edfd6c8e608126eaea6d6dc91f14d7e3b4c0c3bf8a159e50 |
| crates/vb_core/src/policy.rs | 35a38e9a64854755b76b72b12e7b30cd2d4184479dfc7dba19a91551c4633fdc |
| crates/vb_runtime/src/runtime.rs | 999426cb8402d738fc4d801e208c04fd10d8543a6b4e0025d6ea4927a733d793 |
| crates/vb_runtime/src/shard/types.rs | a8f9ff224310810a08dea1a52b6bd97bd4d9dd22c7b34b69700a11fcce9aee45 |
| crates/vb_storage/src/executor/register.rs | 78e07824071d7b7f47da988eb7bdbe6b6755afe795c5c9cf3afde2857b83cd8b |
| crates/vb_runtime/src/admission/parts/chunk_002_records.rs | 4bcf2ea1926fcc751c90fd64c188573e8006362290e9cbc2d45a0b50caa143e3 |
| crates/vb_storage/src/events/event.rs | c8e8700563de158ed2ff04d8dc80705991d4a994d0e9ef1ea3b4f302bb35a8d1 |
| crates/vb_runtime/src/journal/chunk_001.rs | d2f915e6ad5217194c8a9704361628173ccc09c4a8234de4833662fe87b80aff |
