# Contract Specification: vb-core-yaml-e2e-chain

## Context

- Feature: prove the durable core-engine chain from strict YAML source through compile, accepted artifact persistence, Fjall-backed strict runtime execution, events/inspect, replay, and recovery.
- Source facts read: State 2 `codebase-map.md`, `delivery-scope.jsonl`, `baseline-report.md`, `STATE.md`, and bead JSON from `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-yaml-e2e-chain --json`.
- Domain terms:
  - YAML source digest: digest of exact source bytes accepted at the cold boundary.
  - Compiled artifact digest: digest binding accepted serialized IR/artifact bytes to runtime admission.
  - Accepted artifact: storage/runtime envelope containing digest, serialized IR, verification proof, accepted sequence, and required capabilities.
  - Strict runtime admission: runtime path that rejects loose YAML or raw compiled input and admits only persisted accepted artifacts.
  - Journal/events/inspect: Fjall-backed durable evidence surfaces for RunAccepted, RunAdmission, RunFinished, failures, and digest fields.
  - Recovery: restart/replay from persisted headers, source/artifact records, journal events, and snapshots without YAML reparsing.
- Assumptions:
  - Existing package names are `velvet_ballistics`, `vb_compile`, `vb_storage`, `vb_runtime`, and workspace package `velvet-ballistics-workspace`.
  - Downstream states may add tests/proofs, but this contract stage does not write production code, tests, or proof code.
  - `moon ci` remains the release gate; focused Cargo commands are obligation-level evidence commands.
- Open questions:
  - None allowed to remain implicit for State 6 retry. Storage/runtime proof-count parity and raw-IR bypass are release-critical blockers unless downstream implementation repairs them or typed fail-closed evidence proves strict admission rejects them.
  - The existing Kani matrix artifact is not discoverable by `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix`; downstream code/proof integration must wire a crate-local harness or formally waive Kani with owner, expiry, and compensating evidence before approval.

## Preconditions

- PRE-001: YAML-origin execution begins with exact source bytes supplied only to the compile/validation boundary (`vb_compile`/`vb_yaml`/`vb_validate`), never to runtime recovery.
- PRE-002: Strict YAML profile rejects unsupported YAML features before compile: duplicate keys, invalid shape, schema/reference/type/taint/control-flow violations, aliases, anchors, tags, and multi-document streams.
- PRE-003: A source digest claim is accepted only if it equals the digest of the stored YAML source bytes.
- PRE-004: A compiled artifact digest claim is accepted only if it equals the digest of the persisted artifact/IR bytes that runtime will load.
- PRE-005: Strict runtime admission requires a persisted accepted-artifact envelope with sufficient verification gates/proof flags and required capability declarations.
- PRE-006: A run acknowledgement or externally visible strict runtime state is permitted only after durable source/artifact/run header/journal evidence required by the operation has been flushed or reported as persisted by the storage layer.
- PRE-007: Recovery/replay input consists only of persisted run headers, source/artifact records, journal events, snapshots, and compiled workflow/artifact bytes loaded by digest.

## Postconditions

- POST-001: A successful YAML-origin strict run has durable evidence for source storage, compiled/accepted artifact storage, run header, RunAccepted, runtime admission, terminal or suspended runtime state, and inspect/events visibility.
- POST-002: Events and inspect expose enough digest-bound evidence to correlate YAML source digest, compiled/artifact digest, run id, accepted sequence, and final/suspended status.
- POST-003: Restart/replay/recovery reconstructs runtime summary/frame seed from persisted data and does not parse YAML after initial cold-boundary validation/compile.
- POST-004: Corrupt source bytes, corrupt artifact bytes, missing accepted artifact envelope, gate-count/proof mismatch, replay divergence, and source/artifact digest mismatch fail closed with typed errors.
- POST-005: A successful replay/recovery result refines the previously acknowledged journal state: no lost terminal status, slot state, taint, waits/asks/actions, retries, collect state, or run sequence evidence within this bead scope.
- POST-006: No runtime-core path introduced or modified for this bead depends on YAML, JSON, or HTTP parsing; YAML remains a cold compile/input concern.

## Invariants

- INV-001: Source digest binding is stable: stored source bytes for a source digest either match the claimed digest or storage/recovery returns `PayloadDigestMismatch`/`WorkflowSourceDigestMismatch`-class typed failure.
- INV-002: Artifact digest binding is stable: runtime-loaded artifact/IR bytes for an artifact digest either match the claimed digest and accepted proof contract or admission/recovery returns `CompiledIrDigestMismatch`/admission typed failure.
- INV-003: Strict admission never admits raw YAML, loose `WorkflowParts`, missing artifact envelopes, or under-proven artifacts when strict accepted-artifact admission is required.
- INV-004: Journal order is prefix durable: every acknowledged strict run state has a persisted prefix containing the required admission and runtime events before that state is externally visible.
- INV-005: Recovery is YAML-free: after RunAccepted/admission persistence, recovery and replay transitions do not call YAML parser APIs or require YAML source bytes except for digest verification/loading evidence.
- INV-006: Replay is deterministic over persisted evidence: the same persisted source/artifact/journal/snapshot set yields the same recovered summary/frame seed or the same typed fail-closed error.
- INV-007: Events/inspect are faithful projections of Fjall journal state and do not synthesize success absent durable RunAccepted/admission/runtime events.
- INV-008: Source digest and compiled/artifact digest are distinct contract roles even if represented by the same Rust digest type; tests/proofs must not conflate them.

## Error Taxonomy

- Error::StrictYamlRejected - PRE-002 violation at cold compile boundary.
- Error::WorkflowSourceDigestMismatch - source bytes do not match claimed source digest.
- Error::CompiledIrDigestMismatch - compiled IR/artifact bytes do not match claimed artifact digest or RunAccepted digest.
- Error::AcceptedArtifactMissing - strict runtime cannot load accepted artifact envelope by digest.
- Error::AcceptedArtifactInvalid - accepted artifact envelope is malformed, under-proven, or gate/proof flags fail runtime contract.
- Error::CapabilityMismatch - accepted artifact requires capabilities not granted by runtime admission.
- Error::DurabilityFailure - source/artifact/header/journal evidence cannot be persisted before acknowledgement.
- Error::ReplayDivergence - persisted journal/snapshot data diverges from compiled workflow/runtime model.
- Error::CorruptRecoveryData - persisted frame, snapshot, or journal data is corrupt/incomplete and cannot be safely hydrated.
- Error::NoRecoveryData - recovery requested for a run without durable evidence.
- Error::YamlReparseDuringRecovery - recovery path observes or requires YAML parsing after admission; this is a contract violation and must fail the bead.

## Error Evidence Requirements

- ERR-001 / `StrictYamlRejected`: `cargo test -p vb_compile -- --nocapture` must include focused evidence for duplicate keys, invalid shape, schema/reference/type/taint/control-flow violations, aliases, anchors, tags, and multi-document streams returning `StrictYamlRejected`-class errors before compile output is admitted.
- ERR-002 / `WorkflowSourceDigestMismatch`: `cargo test -p vb_storage -- --nocapture` must include source-byte digest mismatch evidence returning `WorkflowSourceDigestMismatch` or `PayloadDigestMismatch`.
- ERR-003 / `CompiledIrDigestMismatch`: `cargo test -p vb_storage -- --nocapture` must include artifact/RunAccepted digest mismatch evidence returning `CompiledIrDigestMismatch`.
- ERR-004 / `AcceptedArtifactMissing`: `cargo test -p vb_runtime -- --nocapture` must include strict admission evidence for missing accepted artifact envelope returning `AcceptedArtifactMissing` or the crate's exact admission variant mapped to this contract error.
- ERR-005 / `AcceptedArtifactInvalid`: `cargo test -p vb_runtime -- --nocapture` must include malformed, under-proven, or gate/proof-count mismatch evidence returning `AcceptedArtifactInvalid`.
- ERR-006 / `CapabilityMismatch`: `cargo test -p vb_runtime -- --nocapture` must include missing capability grant evidence returning `CapabilityMismatch`.
- ERR-007 / `DurabilityFailure`: `cargo test -p velvet_ballistics --test cli_integration -- --nocapture` must include persistence-before-ack failure evidence returning `DurabilityFailure` with no runnable state acknowledged.
- ERR-008 / `ReplayDivergence`: `cargo test -p vb_storage -- --nocapture` must include divergent journal/snapshot evidence returning `ReplayDivergence`.
- ERR-009 / `CorruptRecoveryData`: `cargo test -p vb_storage -- --nocapture` must include corrupt frame/snapshot/journal evidence returning `CorruptRecoveryData` or the crate's exact `CorruptSnapshot`/frame corruption variant mapped to this contract error.
- ERR-010 / `NoRecoveryData`: `cargo test -p vb_storage -- --nocapture` must include no durable run evidence returning `NoRecoveryData`.
- ERR-011 / `YamlReparseDuringRecovery`: `cargo test -p velvet-ballistics-workspace --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture` plus static boundary evidence must prove parser-call sentinel failure or no post-admission parser dependency; success without this evidence is rejected.

## Contract Signatures

- `fn validate_and_compile_yaml(source: YamlSourceBytes) -> Result<CompiledWorkflowArtifact, EngineChainError>`
- `fn persist_source_and_artifact(source: YamlSourceBytes, source_digest: WorkflowDigest, artifact: CompiledWorkflowArtifact) -> Result<AcceptedArtifactRef, EngineChainError>`
- `fn admit_strict_artifact_run(artifact_ref: AcceptedArtifactRef, run_id: RunId, capabilities: CapabilitySet) -> Result<RunAdmission, EngineChainError>`
- `fn append_strict_runtime_event(run_id: RunId, event: JournalEvent) -> Result<DurableEventAck, EngineChainError>`
- `fn inspect_run(run_id: RunId) -> Result<RunInspection, EngineChainError>`
- `fn events_for_run(run_id: RunId) -> Result<RunEventStream, EngineChainError>`
- `fn recover_yaml_origin_run(run_id: RunId) -> Result<RecoveredRuntimeState, EngineChainError>`

## Verus-Owned Clauses

- INV-001, INV-002, INV-006, INV-008: pure digest role equality, mismatch classification, and deterministic recovery summary/state-transition predicates where expressible.
- PRE-003, PRE-004: digest claim predicates.
- POST-004: typed classification for pure mismatch/divergence cases.
- Verus evidence command for current proof surface: `verus verification/verus/yaml_e2e_digest_roles.rs` with expected `verification results:: 6 verified, 0 errors`. This proves the abstract role/classification kernel only; executable shell linkage is owned by required Kani/proptest/E2E obligations and the explicit Verus shell-exclusion waiver in `verification-layers.md`.

## TLA+-Owned Clauses

- PRE-006, POST-001, POST-003, POST-005, INV-003, INV-004, INV-005, INV-007: lifecycle ordering and recovery/replay workflow from YAMLAccepted through Persisted, Admitted, Running, Suspended/Finished/Failed, Restarted, and Recovered.

## Theorem-Owned Clauses

- None mandatory at contract time. Lean/Aeneas/Hax is reserved for a tiny digest-role/refinement theorem only if Verus cannot express distinction between source digest and artifact digest roles without overfitting runtime shell code.

## Non-goals

- UI, generated Rust/codegen parity, and maxperf runtime generation.
- Proving all existing global recovery debt outside this bead's YAML-origin strict chain.
- Replacing downstream test/proof implementation states.
