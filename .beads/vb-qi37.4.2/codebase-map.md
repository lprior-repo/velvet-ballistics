# vb-qi37.4.2 codebase map

Bead: `vb-qi37.4.2` — runtime: Enforce admission gate before run creation.

Workspace verified: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`.
Source checkout was not used for writes.

## Bead contract

- Require accepted artifacts for run creation.
- Reject raw, failed, stale, digest-mismatched, or malformed artifacts before runtime state allocation.
- Valid accepted artifacts proceed without runtime YAML/JSON parsing.

## Primary implementation scope

1. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_runtime/src/admission.rs`
   - Symbols: `REQUIRED_GATE_COUNT`, `ArtifactEnvelopeError`, `AdmissionError`, `AcceptedArtifactStore`, `AlwaysPresentArtifactStore`, `StorageArtifactStore`, `admit_artifact_run`, `admit_run`, `admit_run_with_budget`.
   - Current behavior: strict/journaled `admit_artifact_run` loads an `AcceptedArtifact`, validates gate count and proof flags, and checks exact capability grants.
   - Risk: `REQUIRED_GATE_COUNT` is `15`, but storage artifact submission currently emits `ADMISSION_GATE_COUNT = 2`; this can make real storage-submitted artifacts fail strict runtime admission.
   - Risk: `admit_run`/`admit_run_with_budget` still perform existence-only checks and can bypass full envelope validation if used by future call sites.

2. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
   - Symbols: `handle_submit_inner`, `build_admission`.
   - Current behavior: `build_admission` is called before `take_frame_for`, before `self.runs.insert`, and before `drive_run`; this is the right allocation boundary.
   - Risk: `AdmissionError::ArtifactEnvelopeDecodeFailed` maps to `RuntimeError::AdmissionArtifactInvalid` with a zero digest instead of the rejected artifact digest.
   - Risk: journal `RunSubmitted` is appended before `RunAdmission`; this is acceptable only if no runtime state is allocated and no `RunAccepted`/terminal state is produced on admission failure. Tests should prove this ordering.

3. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
   - Symbols: `Shard::new`, `Shard::new_with_journal`, `Shard::new_with_journal_and_artifact_store`.
   - Current behavior: `Shard::new_with_journal` defaults to `AlwaysPresentArtifactStore::shared()`.
   - Risk: strict/journaled production constructors that use `new_with_journal` may accept dummy artifacts instead of loading from storage. This overlaps dependent bead `vb-core-storage-artifact-store`.

4. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_runtime/src/runtime.rs`
   - Symbols: `Runtime::new_with_journal`, submit methods.
   - Current behavior: runtime construction routes every shard through `Shard::new_with_journal`, therefore inherits `AlwaysPresentArtifactStore` unless a storage-backed constructor is added or called.
   - Risk: CLI strict/journaled paths use this constructor today, so runtime admission may not be storage-backed.

5. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_runtime/src/error/mod.rs`
   - Symbols: `RuntimeError::AdmissionArtifactNotFound`, `AdmissionArtifactInvalid`, `AdmissionCapabilityDenied`, diagnostics/display/equality modules.
   - Current behavior: typed runtime admission errors exist but envelope failure details collapse into `AdmissionArtifactInvalid`.
   - Risk: bead acceptance wants typed diagnostics for raw/unverified/malformed/digest mismatch; current variants may be too coarse unless diagnostics encode enough cause.

## Storage artifact scope

1. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_storage/src/admission.rs`
   - Symbols: `VerificationProof`, `AcceptedArtifact`, `submit_artifact`, `submit_artifact_with_contracts`, `ADMISSION_GATE_COUNT`.
   - Current behavior: `submit_artifact` persists a postcard-encoded `AcceptedArtifact` inside `CompiledIrRecord.ir`.
   - Risk: `ADMISSION_GATE_COUNT` is `2`, while runtime admission requires `15`.
   - Risk: `accepted_at_seq` is set to `EventSeq::new(0)`, not a real journal sequence; dependent `vb-core-atomic-admission` requires real sequence.
   - Risk: relaxed submission persists `gate_count=0`, `durable=false`; strict runtime should reject this as raw/unverified.

2. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_storage/src/artifacts.rs`
   - Symbols: `FjallJournal::compiled_ir` call sites via `list_artifacts`, `remove_artifact`, `artifact_exists`.
   - Current behavior: existence APIs do not validate accepted envelope contents.

3. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_storage/src/error/artifact.rs`
   - Symbols: `ArtifactEnvelopeError`, `ArtifactInvalidSource`.
   - Current behavior: storage has more granular envelope errors than runtime currently exposes.

4. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_storage/src/events.rs`
   - Symbols: `JournalEvent::RunAccepted`, `JournalEvent::RunAdmission`.
   - Current behavior: run admission metadata has its own event separate from run accepted.

## CLI / production entry points

1. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/velvet_ballastics/src/main.rs`
   - Symbols: `run_compiled_workflow`, `runtime_journal_for_mode`, `cmd_run`, `cmd_run_compiled`.
   - Current behavior: `run_compiled_workflow` creates `Runtime::new_with_journal(...)`; no storage-backed accepted artifact store is supplied.
   - Risk: strict/journaled CLI can use durable journal sink but dummy admission store.

2. `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/velvet_ballastics/src/storage.rs`
   - Symbols: `cmd_ipc_serve`, `StorageWorkflowResolver::resolve_workflow`.
   - Current behavior: resolver reads `journal.compiled_ir(digest)` and decodes `record.ir` as `WorkflowParts`.
   - Risk: `submit_artifact` stores `AcceptedArtifact` bytes in `record.ir`; resolver may reject valid accepted artifacts as invalid raw workflow parts. Runtime admission must not parse YAML/JSON, but may need accepted-envelope decoding plus inner IR decode.

## Existing tests and proof assets

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_runtime/src/admission.rs` unit tests cover admission record fields, exact capability grants, and legacy existence-only `admit_run`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/velvet_ballastics/tests/admission_evidence_integration/chunk_002.rs` covers relaxed artifact submission then runtime success, but uses `Runtime::new_with_journal` and does not prove strict storage-backed admission.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/velvet_ballastics/tests/ir_artifact_admission.rs` covers `run-compiled` malformed raw IR rejection, not accepted artifact envelope admission.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` covers `submit_artifact` policy behavior, gate counts, error codes, and accepted artifact persistence.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/verification/verus/capability_artifact_model.rs` is relevant to capability/exact-cardinality admission.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/verification/tla/CapabilityLifecycle.tla` and cfg variants are relevant for capability lifecycle admission properties.

## Recommended downstream work

- Contract owner: define one accepted-artifact v1 gate-count contract shared between `vb_storage` and `vb_runtime`.
- Proof owner: model that failed artifact admission leaves no allocated `RunState` and no runnable/accepted runtime state.
- Test owner: add strict/journaled storage-backed tests for missing artifact, raw `WorkflowParts` bytes, malformed postcard, gate_count 0/2 mismatch, false proof flags, digest mismatch, and valid accepted artifact.
- Implementation owner: introduce or use storage-backed runtime/shard constructor for strict/journaled production entry points; keep `AlwaysPresentArtifactStore` test-only or relaxed-only.

## Open questions

- UNKNOWN: exact go-skill required JSONL schema beyond bead/path/risk/verifier fields was not present in local instructions.
- UNKNOWN: whether gate count should be normalized to 15 in storage, lowered in runtime, or replaced with named gate evidence. Current source disagrees.
- UNKNOWN: whether digest mismatch must be detected by accepted-envelope header validation, inner IR digest validation, or both.
