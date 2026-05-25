# State 2 Codebase Map: vb-core-atomic-admission

bead_id: vb-core-atomic-admission
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission
attempt: State 2 attempt 2

## Bead Scope

Persist strict accepted-run creation as one durable Fjall boundary before acknowledgement. The boundary must include workflow source, compiled IR as `AcceptedArtifact`, run header, `RunAccepted`, and required indexes. Failure injection must leave no partially accepted run. `accepted_at_seq` must record a real journal sequence, not a sentinel.

## Mapped Crates

- `crates/vb_storage`: owns Fjall keyspaces, record codecs, atomic batch API, artifact/source/header/event/index storage, journal sequence, and strict persistence.
- `crates/vb_runtime`: owns runtime admission gate, `RunAdmission`, submit path, journal-before-ack ordering, and storage-backed accepted artifact loading.
- `crates/velvet_ballistics`: owns CLI/operator paths that compile YAML, persist source/artifacts/header/events, and expose inspect/events/readback behavior.
- `crates/vb_core`: owns identifiers and domain types used in persisted records: `RunId`, `WorkflowId`, `WorkflowDigest`, `RuntimePolicy`, `CapabilitySet`, `ActionId`, `StepIdx`.
- `crates/workspace_tests`, `crates/velvet_ballistics/tests`, `fuzz`: likely verification consumers for failure injection, replay/readback, CLI, and admission fuzz evidence.

## Mapped Files And Current Behavior

- `crates/vb_storage/src/journal/core.rs`: `FjallJournal` opens nine keyspaces: workflow source, compiled IR, run header, run event, run snapshot, blob, status index, workflow index, action index. This is the storage aggregate that must host the accepted-run batch.
- `crates/vb_storage/src/batch.rs`: `JournalWriteBatch` wraps `fjall::OwnedWriteBatch` and can stage `put_workflow_source`, `put_compiled_ir`, `put_run_header`, `append_event`, `put_status_index`, `put_workflow_index`, and `put_action_index`, with `strict()` selecting `PersistMode::SyncAll` before `commit()`.
- `crates/vb_storage/src/journal/batch.rs`: `FjallJournal::batch()` exposes the batch builder.
- `crates/vb_storage/src/journal/source.rs`: standalone `put_workflow_source` and `put_compiled_ir` insert into separate keyspaces without an explicit shared atomic admission boundary.
- `crates/vb_storage/src/headers.rs`: standalone `put_run_header` writes the run header and calls `persist_strict()` independently.
- `crates/vb_storage/src/journal/append.rs`: standalone `append_strict_batch` appends events and then calls `persist_strict()`; it is separate from source/artifact/header writes.
- `crates/vb_storage/src/indexes.rs`: standalone index marker writes exist for status, workflow, and action indexes, but do not force durability themselves.
- `crates/vb_storage/src/records.rs`: durable records include `WorkflowSourceRecord`, `CompiledIrRecord`, `RunHeaderRecord`, and `RecordKind::{WorkflowSource, CompiledIr, RunHeader, RunAccepted, RunAdmission, IndexUpdate}`.
- `crates/vb_storage/src/events.rs`: `JournalEvent::RunAccepted { run, seq, workflow }` and `JournalEvent::RunAdmission { run, seq, artifact_digest, granted_capabilities, policy }` are available as durable event variants.
- `crates/vb_storage/src/admission.rs`: `submit_artifact` and `submit_artifact_with_contracts` build `AcceptedArtifact` and persist it as a `CompiledIrRecord`. Current `AcceptedArtifact.accepted_at_seq` is initialized with `EventSeq::new(0)` in both relaxed and strict/journaled paths; current local constant `ADMISSION_GATE_COUNT` is 2.
- `crates/vb_runtime/src/admission.rs`: runtime admission requires `REQUIRED_GATE_COUNT = 15` for strict/journaled accepted artifacts, validates proof flags, loads `AcceptedArtifact` through `StorageArtifactStore`, and builds `RunAdmission` after capability validation.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`: `handle_submit_with_inputs_contracts_and_header_mode` performs `build_admission`, frame allocation, optional `RunSubmitted` append, optional `RunAdmission` append, in-memory insert, and run drive. This is a runtime admission path, but not the storage-level atomic batch for source/artifact/header/RunAccepted/indexes.
- `crates/velvet_ballistics/src/main.rs`: `store_workflow_artifacts` writes workflow source and raw compiled workflow parts as separate records. `cmd_submit` writes workflow source, run header, and `RunAccepted` as separate operations before returning submitted JSON/text; it does not call `submit_artifact` and does not commit one accepted-run batch.
- `crates/velvet_ballistics/src/storage.rs`: `StorageWorkflowResolver` currently decodes `CompiledIrRecord.ir` directly as `WorkflowParts`, which conflicts with strict `AcceptedArtifact` envelope storage unless updated by downstream states.
- `crates/velvet_ballistics/tests/admission_evidence_integration/chunk_001.rs` and `chunk_002.rs`: existing admission evidence covers relaxed artifact persistence and a failing runtime journal before header, but not the full atomic accepted-run batch with failure injection across all required record families.
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`: existing tests cover generic batch persistence/all-or-nothing behavior and `accepted_at_seq` validity expectations, useful as prior evidence but not sufficient for this bead's specific accepted-run boundary.

## Public APIs Likely In Scope

- `vb_storage::FjallJournal::batch`
- `vb_storage::JournalWriteBatch::{put_workflow_source, put_compiled_ir, put_run_header, append_event, put_status_index, put_workflow_index, put_action_index, strict, commit}`
- `vb_storage::{submit_artifact, submit_artifact_with_contracts}`
- `vb_storage::{WorkflowSourceRecord, CompiledIrRecord, RunHeaderRecord, JournalEvent, EventSeq, AcceptedArtifact, VerificationProof}`
- `vb_runtime::admission::{AcceptedArtifactStore, StorageArtifactStore, RunAdmission, admit_artifact_run}`
- `vb_runtime::shard::Shard` submit paths through runtime public wrappers
- CLI surfaces in `crates/velvet_ballistics/src/main.rs`: `run`, `submit`, and storage-backed readback/event inspection paths

## Current Gaps Against Acceptance

- The current CLI submit path writes workflow source, run header, and `RunAccepted` as separate writes; a mid-path failure can plausibly leave a partial accepted run.
- Strict accepted artifact storage is inconsistent: storage admission writes `AcceptedArtifact`, but CLI artifact persistence still stores raw `WorkflowParts` in `CompiledIrRecord.ir`.
- `accepted_at_seq` is currently initialized as `EventSeq::new(0)` during artifact submission rather than being bound to the real `RunAccepted` journal sequence for the accepted run.
- Runtime strict admission expects 15 gates, while storage admission currently has a local 2-gate constant; this is a dependency/risk shared with `vb-core-proof-15-gate` and `vb-core-accepted-artifact-format`.
- `StorageWorkflowResolver` expects raw `WorkflowParts`; strict accepted artifact envelopes need explicit decode/readback behavior or a separate resolver path.
- Existing generic batch tests do not prove this specific all-or-nothing set: workflow source, accepted artifact, run header, `RunAccepted`, status/workflow/action indexes.

## Suggested Delivery Scope

- Introduce or route through a storage-level accepted-run commit API in `vb_storage` that stages every required record in one `JournalWriteBatch::strict().commit()`.
- Ensure the accepted artifact stored in compiled IR is the stable `AcceptedArtifact` envelope, not raw `WorkflowParts`, for strict admission paths.
- Allocate/bind the `RunAccepted` sequence before committing the artifact envelope so `AcceptedArtifact.accepted_at_seq` equals the durable event sequence.
- Route CLI strict `submit`/`run` accepted paths through that storage API before acknowledgement.
- Preserve relaxed or non-durable paths only if explicitly outside strict admission; do not add backward compatibility for strict raw artifacts.
- Add failure injection tests around every stage in the batch construction/commit path and restart/readback assertions that no partial accepted run is visible after failure.

## Risks

- Release-critical durability risk: acknowledging after split writes can strand source/header/event without a matching accepted artifact or indexes.
- Cross-bead schema risk: `vb-core-accepted-artifact-format` and `vb-core-proof-15-gate` are open blockers for final strict artifact semantics.
- Migration risk: existing read paths and tests may assume raw `WorkflowParts` in `compiled_ir`.
- Sequencing risk: `EventSeq::new(0)` may be valid for first event but is not proof that `accepted_at_seq` came from the committed journal event.
- Failure-injection risk: Fjall `OwnedWriteBatch` gives atomic commit, but code can still fail before staging all records; tests must prove nothing is visible on construction errors and failed commit.
- Dependency risk: no Cargo dependency changes appear necessary for the mapped scope; changing dependency files would trigger supply/dependency gates.

## Evidence Collected

- Read `.beads/vb-core-atomic-admission/STATE.md` and `.beads/vb-core-atomic-admission/baseline-report.md` in isolated workspace.
- Ran `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-atomic-admission --json` from isolated workspace; bead status is `in_progress` and scope/acceptance matches this map.
- Searched isolated workspace for `AcceptedArtifact`, `RunAccepted`, `run_header`, `workflow_source`, `accepted_at_seq`, `Fjall`, `WriteBatch`, `submit_artifact`, `RunHeader`, and admission APIs.
- Read mapped files listed above from `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission` only.
