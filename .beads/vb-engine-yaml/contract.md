# Contract Specification: vb-engine-yaml

## Context

- Feature: engine-only durable acceptance root for strict YAML authoring through validation, compile/lowering, accepted artifact admission, bounded runtime execution, Fjall/Postcard persistence, recovery/replay, direct API, IPC, CLI/operator diagnostics, and engine-scoped quality evidence.
- Source artifacts: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-engine-yaml --json`, `.beads/vb-engine-yaml/codebase-map.md`, `.beads/vb-engine-yaml/delivery-scope.jsonl`, `/velvet-ballistics-MASTER.md` copy in this isolated workspace.
- Explicit exclusions: UI, generated Rust/codegen parity, full maxperf generated-mode completion.
- Assumptions: existing child/dependency beads own implementation; this State 3 artifact defines the acceptance contract and verification obligations only.
- Open questions: exact Moon subtask names for formal-only lanes are not established in State 2; obligations use known file-level commands or `moon ci` where repository orchestration owns evidence.

## Domain Terms

- Cold authoring: YAML parsing, validation, diagnostics, source maps, and compile-time lowering before runtime admission.
- Numeric IR: compiled workflow model using numeric IDs for workflows, steps, slots, expressions, actions, accessors, constants, symbols, lists, objects, blobs, and sequence numbers.
- Accepted artifact: digest-bound, verified artifact envelope that is the only unit of trust for admission, persistence, replay, and recovery.
- Strict admission: production path that persists required source/artifact/header/acceptance records before acknowledging run acceptance.
- Runtime core: `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, and generated workflow execution surfaces.
- Durable evidence chain: workflow source, compiled IR/accepted artifact, run header, journal events, snapshots, blobs, indexes, inspect data, replay/recovery reports.

## Preconditions

- PRE-001: Input YAML for strict engine acceptance is provided only to cold-path parser/validator/compiler APIs, never to runtime-core crates.
- PRE-002: A strict run can be admitted only from a structurally valid numeric IR wrapped in an accepted artifact with digest, verification proof envelope, resource contract, idempotency evidence, and capability evidence.
- PRE-003: Strict durability mode requires Fjall keyspaces and Postcard/envelope encoders for workflow source, compiled IR, run header, journal events, snapshots, blobs, and indexes before acknowledgement.
- PRE-004: Runtime execution starts only after resource limits for steps, slots, constants, accessors, expressions, queues, frame pools, retries, fanout, trace rings, expression stacks, IPC frames, payloads, blobs, and journal batches are explicit and checked.
- PRE-005: Recovery/replay begins only from persisted source/artifact digests, run headers, snapshots, and journal records; YAML source text is not reparsed for existing runs.
- PRE-006: Direct API, binary IPC, and CLI operator surfaces must route through typed commands and accepted artifacts; text-command, JSON, HTTP, or loose-YAML runtime submission is outside strict engine acceptance.

## Postconditions

- POST-001: Strict YAML authoring accepts and rejects the same v1 language shapes across `vb_yaml`, `vb_validate`, and `vb_compile`, with stable typed diagnostics for invalid shapes.
- POST-002: Accepted artifacts are the only production trust boundary for runtime admission; raw `WorkflowParts`, loose YAML, dummy proof stores, or unchecked `CompiledWorkflow` values cannot satisfy strict admission.
- POST-003: Strict admission durably persists source/artifact/header/acceptance/index evidence atomically or fails before acknowledgement with typed errors and no partial runnable state.
- POST-004: Runtime-core crates remain free of YAML interpretation, JSON parsing/routing, HTTP handling, text command protocols, runtime string reference resolution, and unbounded hot-path structures.
- POST-005: Deterministic runtime execution is bounded, typed, and shard-owned; suspension, retry, action, wait, ask, cancel, inspect, and terminal transitions preserve journal/replay semantics.
- POST-006: Recovery/replay reconstructs acknowledged state from snapshots plus tail journal or full journal, detects digest/semantic divergence, and fails closed on corrupt, incomplete, or mismatched durable evidence.
- POST-007: CLI/operator diagnostics expose typed validation, compile, admission, runtime, storage, recovery, and IPC outcomes without becoming a runtime text protocol.
- POST-008: Engine-scope evidence includes canonical CI plus focused proof/test/fuzz/Miri/coverage/mutation/supply/perf evidence required by this contract and dependency closure.

## Invariants

- INV-001: YAML is cold authoring only; no runtime-core path interprets YAML, parses JSON, serves HTTP, or routes text commands.
- INV-002: Final executable runtime state is numeric and handle-based; runtime does not resolve string references dynamically.
- INV-003: Accepted artifact digest, source digest, policy digest, action ABI digest, and verification proof envelope remain immutable for an accepted run.
- INV-004: Strict acknowledgement happens only after required durable records are persisted according to strict profile semantics.
- INV-005: Journal sequence numbers are monotonic per run and replay preserves acknowledged state exactly.
- INV-006: Resource contracts are enforced before allocation/execution can exceed bounds; no silent truncation, unbounded queue, unbounded retry, unbounded fanout, or unchecked payload growth is allowed.
- INV-007: Idempotency and capability gates are real derived evidence; default-true or missing gates reject strict artifacts.
- INV-008: Recovery never reparses YAML for existing runs and never silently continues after corrupt, incomplete, mismatched, or semantically divergent durable evidence.
- INV-009: UI and generated Rust/codegen parity are non-goals for this bead and cannot block engine-only acceptance unless they leak into runtime-core contracts.

## Error Taxonomy

- EngineYamlError::ColdYamlViolation - YAML reaches runtime-core or strict runtime admission without cold compile/validation.
- EngineYamlError::UnsupportedRuntimeProtocol - JSON, HTTP, text command, or loose YAML runtime ingress is attempted.
- EngineYamlError::InvalidAuthoringShape - YAML violates v1 parser/profile/schema/trigger/step/value/reference contract.
- EngineYamlError::ValidationDrift - parser, validator, and compiler disagree on accept/reject behavior for the same shape.
- EngineYamlError::InvalidNumericIr - IR structural validation, bounds, references, terminals, accessors, or action/resource contracts fail.
- EngineYamlError::ArtifactNotAccepted - raw IR, loose YAML, malformed artifact, legacy artifact, dummy proof, or missing gate evidence is used for strict admission.
- EngineYamlError::DigestMismatch - source/artifact/policy/action ABI digest does not match persisted or replayed evidence.
- EngineYamlError::CapabilityDenied - accepted artifact lacks required capability certificate or runtime dispatch denies capability.
- EngineYamlError::NonIdempotentReplayBlocked - replay/retry would duplicate an unsafe side effect.
- EngineYamlError::DurabilityBeforeAckFailed - strict persistence cannot atomically record required pre-ack evidence.
- EngineYamlError::ResourceLimitExceeded - any compile/runtime/IPC/storage configured bound would be exceeded.
- EngineYamlError::ReplayDiverged - deterministic replay differs from acknowledged journal/snapshot/artifact semantics.
- EngineYamlError::RecoveryIncomplete - durable snapshot/journal/header/index evidence cannot hydrate required live frame state.
- EngineYamlError::CorruptRecord - envelope, CRC, digest, schema, record kind, EOF, or Postcard decode validation fails.
- EngineYamlError::Backpressure - bounded direct API/IPC/runtime/storage queue is full and rejects without blocking or unbounded buffering.
- EngineYamlError::OperatorEvidenceMissing - CLI/operator path does not expose typed evidence required for acceptance.

## Contract Signatures

- `fn parse_workflow_source(bytes: &[u8]) -> Result<WorkflowSource, EngineYamlError>`
- `fn validate_authoring(source: &WorkflowSource) -> Result<ValidatedWorkflow, EngineYamlError>`
- `fn compile_source(source: &WorkflowSource) -> Result<CompiledWorkflow, EngineYamlError>`
- `fn accept_artifact(compiled: CompiledWorkflow, proof: VerificationProof, contract: ResourceContract) -> Result<AcceptedArtifact, EngineYamlError>`
- `fn persist_accepted_run(request: AcceptedRunRequest) -> Result<RunAcceptedEvidence, EngineYamlError>`
- `fn submit_direct(request: SubmitRunRequest) -> Result<RunAcceptedEvidence, EngineYamlError>`
- `fn submit_ipc(frame: BinaryIpcFrame) -> Result<IpcResponse, EngineYamlError>`
- `fn drive_deterministic(run: RunId, budget: StepBudget) -> Result<EngineSignal, EngineYamlError>`
- `fn recover_run(run: RunId) -> Result<RecoveredRunFrame, EngineYamlError>`
- `fn replay_run(run: RunId) -> Result<ReplayReport, EngineYamlError>`
- `fn inspect_run(run: RunId) -> Result<OperatorEvidence, EngineYamlError>`

## Verus-Owned Clauses

- PRE-004, INV-002, INV-005, INV-006: numeric IDs, checked access, budgets, sequence monotonicity, taint lattice, step state transitions, value store invariants, recovery model invariants.
- INV-007: proof gate lattice and capability/idempotency evidence absence implies rejection.

## TLA+-Owned Clauses

- INV-003, INV-004, INV-005, INV-008, POST-003, POST-005, POST-006: admission lifecycle, strict persistence-before-ack, recovery/replay state machine, capability lifecycle, IPC/direct submission and backpressure/order semantics.

## Theorem-Owned Clauses

- None required for State 3. Verus owns Rust-local pure obligations; TLA+ owns temporal lifecycle obligations. Lean/Aeneas/Hax may be introduced later only for a tiny extracted digest/artifact/refinement kernel if Verus cannot express it.

## Non-goals

- UI delivery.
- Generated Rust/codegen parity.
- Full maxperf generated-mode completion.
- Distributed replication, quorum, leader election, or control plane.
- HTTP/JSON runtime adapters.
- New production code, tests, or proof code in State 3.
