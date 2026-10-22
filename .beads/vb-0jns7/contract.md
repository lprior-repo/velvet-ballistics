# Scoped Grant Lifecycle Contract (vb-0jns7)

Source-bound contract against existing opaque `RegisterGrant` authority and actual runtime call paths. Defines missing lifecycle fields, trusted inputs, and behavioral acceptance matrix. No product edits.

## Domain Model

### Existing Authority (verified via source read)

- `crates/vb_storage/src/executor/authority.rs` — `RegisterGrant` opaque struct: `artifact` (WorkflowDigest), `pack` (ActionPackDigest), `ticket` (ActionTicket), `scope` (CapabilityScopeSpec), `timeout_ms` (u64). `validate()` checks ticket match, valid key, attempt bounds. Explicit comment: "does not implement expiry/revocation/remote grant lifecycle."
- `crates/vb_storage/src/executor/catalog.rs` — `ExecutorCatalog::issue_register_grant()` builds grant from artifact, ticket, scope. `validate_receipt()` rechecks receipt against artifact/ticket.
- `crates/vb_storage/src/executor/register.rs` — `FjallRegister::execute()` performs effect or returns idempotent receipt; `reconcile()` reads receipt, never writes.
- `crates/vb_runtime/src/runtime/registered_executor.rs` — `Runtime::execute_registered_action()` calls `registered_context()`, then `adapter.execute()`. `reconcile_registered_action()` calls `adapter.reconcile()`.
- `crates/vb_runtime/src/shard/lifecycle/registered_completion.rs` — `registered_action_context()` verifies artifact, node, action, input slot, output slot, preflights completion.
- `crates/vb_core/src/action_pack.rs` — `CapabilityScopeSpec` = exact numeric target `(namespace: u64, key: u64)`, no wildcards.
- `crates/vb_core/src/capability.rs` — `Capability` = `{name, action}`, `CapabilitySet` = bounded grants.

### Missing Lifecycle Fields (this contract)

Add to `RegisterGrant` or wrap with explicit constructor:

- `issuer: GrantIssuer` — newtype u64 identifying trusted issuer actor.
- `subject: GrantSubject` — newtype u64 identifying authorized subject.
- `grant_id: GrantId` — newtype u128 unique grant identifier for lifecycle tracking.
- `issued_at: JournalTime` — typed u64 journal time (not ambient clock) of issuance.
- `expires_at: Option<JournalTime>` — typed u64 journal time of expiry; None = no expiry.
- `revoked_at: Option<JournalTime>` — typed u64 journal time of revocation; None = not revoked.
- `delegable: bool` — whether grant can be delegated; first slice: false.
- `break_glass: bool` — manual override marker; first slice: false.
- `approval_receipt_digest: Option<ReceiptDigest>` — bound approval receipt; first slice: None.

## Trust Ownership

- Grant is actor-owned value passed by value to executor and runtime.
- Issuer identity established at construction time; no runtime re-derivation.
- Subject identity checked at dispatch time against ticket/run context.
- Time/revocation are trusted typed facts journaled, not ambient.

## Old/New Authority Boundary

- `RegisterGrant::validate()` retains ticket/artifact binding.
- New `GrantLifecycle::check()` adds issuer/subject/time/revocation/delegation checks.
- `ExecutorCatalog::issue_register_grant()` becomes `issue_lifecycle_grant()` accepting all lifecycle fields.
- Existing `RegisterGrant` fields remain; no renumbering of wire tags.

## Dispatch vs Reconcile Separation

- **Dispatch** (`execute_registered_action`): calls `GrantLifecycle::check()` then `adapter.execute()`.
- **Reconcile** (`reconcile_registered_action`): calls `adapter.reconcile()` only; no lifecycle check. Historical receipt is preserved.
- Reconciliation never re-evaluates old authority into new effects.

## Failure Atomics and Durability

- `FjallRegister::execute()` atomic write-or-identical-receipt semantics unchanged.
- Lifecycle check failure returns `ExecutorError::Unauthorized` before any target write.
- Receipt contains all lifecycle fields for durable historical evidence.
- On restart/replay, historical receipts are read-only; no effect re-execution.

## Explicit Narrow First Slice

1. Add lifecycle fields to `RegisterGrant` struct (or new `LifecycleGrant` wrapper).
2. Implement `GrantLifecycle::check()` with all acceptance criteria.
3. Wire into `execute_registered_action()` before adapter call.
4. Reconcile path unchanged (read-only).
5. No delegation/break-glass implementation in first slice (fields exist but checked simple).

## Behavioral Acceptance Matrix

| # | Scenario | Expected |
|---|----------|----------|
| 1 | Wrong issuer | Reject before dispatch, `Unauthorized` |
| 2 | Wrong subject | Reject before dispatch, `Unauthorized` |
| 3 | Wrong artifact | Reject (existing `validate()`), `Unauthorized` |
| 4 | Wrong ticket | Reject (existing `validate()`), `Unauthorized` |
| 5 | Wrong scope (overgrant) | Reject with `strict_exact`, `Unauthorized` |
| 6 | Missing grant | Reject or suspend at explicit gate, `Unauthorized` |
| 7 | Expired grant (`expires_at` < live time) | Reject before dispatch, `Expired` |
| 8 | Not-yet-valid grant (`issued_at` > live time) | Reject before dispatch, `NotYetValid` |
| 9 | Revoked grant (`revoked_at` is Some, < live time) | Reject before dispatch, `Revoked` |
| 10 | Delegation violation (`delegable` false, delegated) | Reject before dispatch, `Unauthorized` |
| 11 | Break-glass without approval receipt | Reject if `approval_receipt_digest` required |
| 12 | Valid minimal grant | Dispatch, execute, return receipt with lifecycle fields |
| 13 | After-schedule revoke, then reconcile | Reconcile reads historical receipt, no new effect |
| 14 | Restart/replay historical receipt | Zero effects, receipt preserved |
| 15 | Approval cannot broaden authority | Narrow scope enforced regardless of approval |

## API Changes

- `issue_register_grant(artifact, ticket, scope)` → `issue_lifecycle_grant(artifact, ticket, scope, issuer, subject, grant_id, issued_at, expires_at, revoked_at, delegable, break_glass, approval_receipt_digest)`
- New `GrantLifecycle::check(grant, live_time, revocation_time)` — pure check returning `Result<(), ExecutorError>`.
- `execute_registered_action()` calls `check()` before `adapter.execute()`.
- No serde defaults; wire/schema version bumped for grant struct.

## Wire/Schema Versioning

- `RegisterGrant` postcard schema changes; bump `Metadata.schema` in `FjallRegister`.
- Old receipts readable via migration path; new receipts include lifecycle fields.
- No field renumbering; additive fields only.

## Source Map

```json
{
  "authority": {
    "file": "crates/vb_storage/src/executor/authority.rs",
    "hash": "ceaee026dfb5ae82c2bf1c78b77357549d161bc010d73002b38516fd9470e6c4",
    "symbols": ["RegisterGrant", "RegisterGrant::validate"]
  },
  "catalog": {
    "file": "crates/vb_storage/src/executor/catalog.rs",
    "hash": "2266fbdbee717ffc455b70a8b25adece0ffed0aa92ce24ca07f07027c217c2d4",
    "symbols": ["ExecutorCatalog", "ExecutorCatalog::issue_register_grant", "ExecutorCatalog::validate_receipt"]
  },
  "register": {
    "file": "crates/vb_storage/src/executor/register.rs",
    "hash": "78e07824071d7b7f47da988eb7bdbe6b6755afe795c5c9cf3afde2857b83cd8b",
    "symbols": ["FjallRegister", "FjallRegister::execute", "FjallRegister::reconcile"]
  },
  "runtime_executor": {
    "file": "crates/vb_runtime/src/runtime/registered_executor.rs",
    "hash": "42cebc4f6cbd4b2d5e1d70a0d730fb30064cbd13924630f95da235e6ef7acdfb",
    "symbols": ["Runtime::execute_registered_action", "Runtime::reconcile_registered_action", "Runtime::registered_context"]
  },
  "registered_completion": {
    "file": "crates/vb_runtime/src/shard/lifecycle/registered_completion.rs",
    "hash": "e2b041e1ce48911842ac8c7388ec76fed3cd9ff57aa1b74f892c87840dd0de5d",
    "symbols": ["RegisteredActionContext", "Shard::registered_action_context"]
  },
  "registered_verification": {
    "file": "crates/vb_runtime/src/admission/registered_verification.rs",
    "hash": "34d4e5a4ba2710b40603973948ce64bb374def9a947c45ed45c219b43a92a1cf",
    "symbols": ["verify_stored_artifact"]
  },
  "capability": {
    "file": "crates/vb_core/src/capability.rs",
    "hash": "f1e0ccb0c4d26044f824343623465e7e41434ce51d9cd1e1b6e9ad4d788b17e8",
    "symbols": ["Capability", "CapabilitySet"]
  },
  "action_pack": {
    "file": "crates/vb_core/src/action_pack.rs",
    "hash": "93ea312ace73bc2c508e2490ad006ca9c0a48bbd8640fd9af50094107bf2960f",
    "symbols": ["CapabilityScopeSpec", "TimeoutPolicy", "ActionPack", "ReceiptDigest"]
  }
}
```