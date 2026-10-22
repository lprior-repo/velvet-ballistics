# vb-ny068 Contract: Pinned Admission/Replay Policy vs Live-Effect Policy

## Domain

Master §§77.3,77.11. Separates pinned admission/replay policy (bound to artifact at run admission) from current live-effect policy evaluated at action dispatch. Adds monotonic emergency-deny overlay that can suspend future effects without altering historical journal.

## New Types

```vb_core::policy::PolicyDigest
```
Blake3 digest identifying a policy configuration. Wraps existing WorkflowDigest.

```vb_core::policy::DecisionCode
```
Typed outcome of a policy decision: `Allowed`, `Denied`, `Suspended`.

```vb_core::policy::PolicyLayer
```
Which policy layer produced the decision: `Admission`, `Replay`, `LiveEffect`, `EmergencyDeny`.

```vb_runtime::policy::PolicyDecision
```
Complete decision record journaled at the effect boundary. Contains policy layer, decision code, policy digest, input digest, and flags for emergency/compatibility influence.

```vb_runtime::policy::PolicyDecisionRecorder
```
Component that evaluates current live-effect policy + emergency overlay and records the decision.

## Production Seam

The policy decision is evaluated at the action outbox boundary, specifically in `prepare_awaiting_action` in `crates/vb_runtime/src/shard/lifecycle/chunk_002_parts/chunk_002_drive_core.rs`. This is called after the engine produces an `AwaitingAction` signal and before the ticket is sent to the executor.

This is the correct seam because:
- The engine is deterministic and produces the same outbox intents on replay
- The executor is the live-effect boundary
- The decision must be recorded before dispatch but after the deterministic computation
- The decision must not affect the deterministic engine computation (replay theorem)

## Legal Transitions

```
Engine produces AwaitingAction(ticket)
    -> Policy decision evaluation at outbox boundary
    -> If Allowed: journal decision, send ticket to executor
    -> If Denied: journal decision, suspend run with typed denial
    -> If Suspended: journal decision, suspend run for external review
```

Replay path:
```
Replay reconstructs journal including policy decisions
    -> Historical decision is restored from journal
    -> No re-evaluation of live policy
    -> No new effects performed
```

## Errors

```vb_runtime::policy::PolicyError
```
- `DigestMismatch`: Policy digest in journal doesn't match expected
- `DecisionCodeUnknown`: Unrecognized decision code during replay
- `EmergencyDenyActive`: Emergency deny overlay is active

## Storage Contract

Each policy decision is recorded as a new `RuntimeJournalEvent` variant `PolicyDecision` with:
- run_id
- policy_layer (PolicyLayer)
- decision_code (DecisionCode)
- policy_digest (PolicyDigest)
- input_digest (PolicyDigest)
- emergency_deny_active (bool)
- compatibility_influence (bool)

The decision is journaled before any live effect is dispatched, ensuring crash consistency.

## Hazards

1. **Replay must not re-evaluate**: The policy decision must be reconstructed from the journal during replay, not re-evaluated. This preserves the replay theorem.

2. **Emergency deny is monotonic**: Once active, emergency deny cannot be cleared without a new artifact admission. This is a policy constraint, not enforced by this bead.

3. **Digest computation must be deterministic**: The input digest must be computed from deterministic ticket fields only (run_id, step, seq, action, attempt).

4. **Relaxed policy does not bypass authorization**: The existing RuntimePolicy enum controls durability/verification, not authorization. Relaxed mode must still evaluate the live-effect policy at dispatch.

5. **No always-allow fallback**: If policy evaluation fails, the effect must be denied, not allowed. This is fail-closed behavior.

## Remaining Work (if not fully complete)

If the full bead cannot be implemented in one slice, the remaining work would be:
- Policy versioning and compatibility algebra (owned by other beads)
- Grant lifecycle and capability scope narrowing (owned by other beads)
- Full emergency deny overlay management API (operator tooling)