# Contract Specification: vb-core-ipc-sync-evidence

## Context
- Bead: `vb-core-ipc-sync-evidence`.
- Feature: prove local binary IPC ingress and runtime synchronization for accepted-artifact submit, bounded ingress, cancel/completion, timer ordering, shutdown drain, slow-client/backpressure, synchronous fanout, and runtime-core format boundaries.
- Repair input: State 6 rejection artifacts `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, and `contract-verification-review.md`.
- Skill authority read: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md`; both are rust-contract version 2.6.0 and require executable obligations, exact commands, TLA+ for temporal behavior, Verus-first for pure Rust-local behavior, and explicit waivers/blockers instead of invented targets.

## Domain Terms
- `IPC frame`: little-endian binary frame with fixed header and bounded postcard payload.
- `SubmitRun`: IPC command carrying a compiled workflow reference or accepted-artifact digest path.
- `strict admission`: runtime admission accepts only artifacts with required evidence and digest agreement.
- `bounded ingress`: finite queue/buffer path with typed full/backpressure/disconnect behavior.
- `terminal race`: cancel/completion/timer/shutdown ordering where at most one deterministic terminal effect mutates state.

## Assumptions and Repair Stance
- This State 3 repair does not write production source, tests, proof/model code, or harnesses.
- Existing executable proof artifacts are limited to `verification/tla/IpcSyncEvidence.tla`, `verification/tla/IpcSyncEvidence.cfg`, `verification/tla/IpcSyncEvidenceCap1.cfg`, `verification/verus/ipc_strict_admission.rs`, `verification/verus/ipc_capacity_bounds.rs`, and `verification/verus/ipc_runtime_transitions.rs`.
- Existing TLA+ configs encode bounded safety/enabledness invariants only. This contract no longer claims true temporal liveness/fairness until proof code is changed to add `PROPERTY` and fairness clauses.
- Existing Verus files are pure models. They are valid only as pure-kernel evidence plus a required refinement-blocker obligation; they do not by themselves prove production linkage.

## Requirements and Contract Clauses
- REQ-IPC-001 / CON-IPC-001: IPC `SubmitRun` or equivalent artifact-digest path SHALL reach strict runtime admission before runtime submit is accepted.
- REQ-IPC-002 / CON-IPC-002: bounded IPC ingress and runtime shard queues SHALL reject full submissions with typed backpressure and SHALL NOT block, allocate unboundedly, or silently drop work.
- REQ-IPC-003 / CON-IPC-003: cancel versus completion races SHALL be deterministic: at most one terminal effect mutates state and stale loser effects are rejected or ignored without mutation.
- REQ-IPC-004 / CON-IPC-004: timer ordering races SHALL be deterministic and SHALL NOT resurrect canceled or terminal runs.
- REQ-IPC-005 / CON-IPC-005: graceful shutdown SHALL drain accepted in-flight work according to the runtime contract and reject new work deterministically after shutdown admission closes.
- REQ-IPC-006 / CON-IPC-006: slow-client IPC behavior SHALL stay within explicit payload/read/write/queue/connection limits and fail with typed backpressure or safe disconnect.
- REQ-IPC-007 / CON-IPC-007: runtime orchestration SHALL remain synchronous and shard-owned; scoped hot paths SHALL NOT introduce task-per-step or unbounded async fanout. Temporal coverage is bounded safety/enabledness only until TLA+ liveness repair.
- REQ-IPC-008 / CON-IPC-008: YAML, JSON, and HTTP SHALL NOT enter the hot runtime core for scoped `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc` paths.

## Preconditions
- PRE-001: callers provide syntactically valid binary IPC headers before payload allocation.
- PRE-002: payload length and bounded-payload limits are known before decoding or queue admission.
- PRE-003: `SubmitRun` has resolver-backed compiled workflow evidence or an equivalent accepted-artifact digest path.
- PRE-004: runtime shard command queues are constructed with non-zero finite capacity.
- PRE-005: terminal race commands identify an existing run/action/timer/shutdown scope.
- PRE-006: tests/proofs that claim strict admission use rejecting or real accepted-artifact semantics, not only always-accepting stores.

## Postconditions
- POST-001: valid accepted submit frames enqueue exactly one runtime submit command or return a typed admission error.
- POST-002: queue-full conditions return `IpcError::Full`, `RuntimeError::QueueFull`, or typed public equivalents without blocking or dropping accepted work.
- POST-003: cancel/completion races end in at most one durable terminal outcome; stale loser effects do not mutate state.
- POST-004: timer events fire only while eligible and do not mutate terminal/canceled state.
- POST-005: shutdown produces inspectable drained/rejected state and does not accept new work after shutdown closes admission.
- POST-006: slow or malicious clients cannot grow memory beyond explicit bounded limits before typed error or disconnect.
- POST-007: scoped source has no unclassified task-per-step spawn/fanout, unbounded hot-path buffer, or YAML/JSON/HTTP runtime-core dependency.

## Invariants
- INV-001: public IPC bounded types are canonical and do not diverge between facade and split modules.
- INV-002: every queue has fixed capacity and full behavior is non-blocking typed backpressure.
- INV-003: a run/action has at most one accepted terminal outcome.
- INV-004: runtime state transitions preserve legal lifecycle/state-machine constraints and reject illegal transitions without mutation.
- INV-005: timers are ordered by the runtime timer contract and cannot fire after cancellation/terminal cleanup.
- INV-006: shutdown is monotonic: once closing/closed, admission never reopens within the same runtime instance.
- INV-007: all hot runtime-core inputs are binary/typed Rust data, not YAML/JSON/HTTP.

## Error Taxonomy
- ERR-001 `IpcError::PayloadTooLarge`: frame or payload exceeds configured maximum.
- ERR-002 `IpcError::Full`: IPC memory ingress/backpressure queue is full.
- ERR-003 `IpcError::Disconnected`: ingress or client channel disconnected safely.
- ERR-004 `IpcResponse::WorkflowResolutionRequired`: `SubmitRun` lacks a required resolver.
- ERR-005 `IpcResponse::WorkflowDigestMismatch`: supplied digest does not match resolved compiled workflow.
- ERR-006 `IpcResponse::RuntimeError`: runtime admission/queue/lifecycle failure returned through IPC response.
- ERR-007 `RuntimeError::QueueFull`: shard command queue is full.
- ERR-008 `AdmissionError`: strict accepted-artifact admission rejects missing or invalid evidence.
- ERR-009 `InvalidStateTransition`: cancel/completion/timer/shutdown command would violate lifecycle rules.
- ERR-010 `SlowClientBackpressure`: typed disconnect or bounded write rejection until a public variant is selected.

## Contract Signatures
- `fn decode_frame(header_bytes: &[u8], payload_bytes: &[u8], max_payload: MaxPayloadBytes) -> Result<IpcFrame, IpcError>`
- `fn submit_ingress(ingress: &MemoryIngress, frame: IngressFrame) -> Result<(), IpcError>`
- `fn dispatch_submit_run(runtime: &Runtime, resolver: &dyn WorkflowResolver, frame: IpcFrame) -> Result<IpcResponse, IpcResponse>`
- `fn submit_to_runtime(runtime: &Runtime, accepted_artifact: AcceptedArtifactDigest) -> Result<RunId, RuntimeError>`
- `fn enqueue_shard_command(queue: &ShardCommandQueue, command: ShardCommand) -> Result<(), RuntimeError>`
- `fn apply_terminal_event(state: RuntimeState, event: TerminalEvent) -> Result<RuntimeState, RuntimeError>`
- `fn apply_timer_event(state: RuntimeState, timer: TimerEvent) -> Result<RuntimeState, RuntimeError>`
- `fn shutdown_graceful(runtime: &Runtime) -> Result<ShutdownReport, RuntimeError>`

## Verus-Owned Clauses
- CON-IPC-001: pure strict-admission witness and digest agreement, checked by `verus verification/verus/ipc_strict_admission.rs`, plus required production-refinement blocker `REFINE-IPC-001`.
- CON-IPC-002: non-zero finite capacity and no underflow/overflow in capacity arithmetic, checked by `verus verification/verus/ipc_capacity_bounds.rs`, plus required production-refinement blocker `REFINE-IPC-002`.
- CON-IPC-003: pure single-terminal-winner predicate, checked by `verus verification/verus/ipc_runtime_transitions.rs`, plus required production-refinement blocker `REFINE-IPC-003`.
- CON-IPC-004: pure timer eligibility/no-resurrection predicate, checked by `verus verification/verus/ipc_runtime_transitions.rs`, plus required production-refinement blocker `REFINE-IPC-004`.
- CON-IPC-005: pure shutdown monotonicity predicate, checked by `verus verification/verus/ipc_runtime_transitions.rs`, plus required production-refinement blocker `REFINE-IPC-005`.
- CON-IPC-008: static dependency/path policy; Verus waiver is valid because the property is about source/dependency classification, not a Rust-local pure state transition.

## TLA+-Owned Clauses
- CON-IPC-001 through CON-IPC-007 are TLA+-owned as bounded safety/enabledness claims using existing configs. Real liveness/fairness remains blocked by `BLOCK-TLA-LIVENESS` until `PROPERTY` and fairness clauses are added to the proof model.

## Theorem-Owned Clauses
- None required. Lean/Aeneas/Hax is waived unless a later reviewer identifies a tiny theorem-only lattice beyond TLA+/Verus.

## Non-goals
- No production implementation, test code, proof/model code, or harness code in this repair.
- No claim that current TLA+ artifacts prove temporal liveness/fairness.
- No claim that current pure Verus artifacts prove production linkage without the required refinement blockers.
- No performance speedup claim beyond boundedness/no-unbounded-fanout evidence.
