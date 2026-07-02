# Domain Model Review: vb-core-ipc-sync-evidence

## Verdict
STATUS: REPAIRED FOR INDEPENDENT REVIEW

This is not an approval artifact. The independent `contract-verification-reviewer` must approve or reject the repaired State 3 contract before downstream states consume it.

## Model Boundaries
- `vb_ipc` owns binary frame validation, bounded memory ingress, socket polling, resolver-backed dispatch, and typed IPC responses.
- `vb_runtime` owns shard command queues, runtime admission, lifecycle events, timer handling, shutdown, and synchronous command processing.
- `vb_core` owns shared identifiers, workflow digest types, capabilities, frame/state vocabulary, and runtime policy data.
- `vb_storage` is only in scope where accepted-artifact evidence requires durable artifact store semantics.

## Type Model
- `QueueCapacity` and `MaxPayloadBytes` must be non-zero finite values.
- `BoundedPayload` is a parsed/validated payload, not an unchecked byte vector.
- `IngressFrame` must contain a run ID, workflow/digest identity, and bounded payload.
- `IpcFrameHeader` is valid only when magic, version, command, reserved bytes, and payload length agree with the protocol.
- `ShardCommandQueue` is the runtime-owned bounded command admission surface.
- `AcceptedArtifactStore` separates resolver convenience from strict runtime admission.
- `RuntimeState`, `TerminalEvent`, `TimerEvent`, and `ShutdownState` remain separate concepts; conflating them hides illegal transitions.

## Repaired Contract Semantics
- TLA+ now explicitly claims only bounded safety/enabledness for existing `verification/tla/IpcSyncEvidence.*` artifacts. True liveness/fairness/deadlock freedom is a named blocker, not a claimed proof.
- Verus now maps CON-IPC-003 through CON-IPC-005 to exact existing command `verus verification/verus/ipc_runtime_transitions.rs`.
- CON-IPC-007 now has a canonical TLA+ obligation for bounded fanout abstraction plus `SCAN-IPC-007` for source-level fanout.
- Pure Verus-to-production linkage is represented by explicit refinement blocker obligations `REFINE-IPC-001` through `REFINE-IPC-005`.

## Illegal States to Make Unrepresentable or Reject Early
- Zero queue capacity.
- Payload length larger than configured maximum.
- Header payload length disagreeing with actual payload bytes.
- `SubmitRun` accepted without digest agreement and strict admission evidence.
- Runtime enqueue succeeding after command queue full.
- Completion after cancel mutating terminal canceled state.
- Timer firing after terminal cleanup.
- Shutdown reopening admission.
- Duplicate IPC bounded type definitions diverging from canonical modules.
- Hot runtime core accepting YAML, JSON, or HTTP data.

## Drift Risks and Active Blockers
- Current IPC submit evidence may prove resolver-backed runtime enqueue but not production strict-admission linkage; tracked by `REFINE-IPC-001`.
- Existing Verus proofs are pure; production adapters/refinement maps remain required for final closure.
- Existing Loom commands do not compile under `--cfg loom`; tracked by `LOOM-IPC-002` through `LOOM-IPC-005`.
- `cargo test -p vb_ipc slow_client` selects zero tests; tracked by `PROP-IPC-006`.
- Static scans need exhaustive per-match classification; tracked by `SCAN-IPC-007` and `SCAN-IPC-008`.
- Existing TLA+ configs do not prove true temporal liveness/fairness; tracked by `BLOCK-TLA-LIVENESS`.

## Required Refinement Relations
- Binary frame bytes refine to `IpcFrame` only through header and payload validation.
- `IpcFrame::SubmitRun` refines to exactly one runtime submit command or typed rejection.
- Accepted artifact digest refines to runtime admission only when strict evidence is present.
- Runtime event traces refine to TLA+ actions by run ID, command kind, and state transition.
- Pure Verus transition predicates refine to production state mutation APIs before final proof closure.

## Open Questions for Later States
- What exact public error variant should represent bounded slow-client write rejection if safe disconnect is insufficient?
- Which accepted-artifact store implementation should tests use to prove strict rejection, not only always-accepting behavior?
- Should IPC facade parity be blocked on dependency beads, or can this bead prove behavior through public re-exports while duplicate definitions remain?
