# Codebase map: vb-core-cli-accepted-path

## Bead
- id: `vb-core-cli-accepted-path`
- title: `cli/runtime: Route YAML run and submit through accepted artifacts`
- source checkout: `/home/lewis/src/velvet-ballistics`
- isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path`

## Request and acceptance signal
Bead requires strict YAML `run`, `submit`, and strict direct run paths to persist verified YAML source and accepted artifacts before runtime admission. Runtime admission must bind by artifact digest through storage-backed admission; loose YAML, raw `CompiledWorkflow`, raw `WorkflowParts`, and unverified compiled input must not bypass strict mode.

## Relevant production paths read

### CLI front door: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/velvet_ballistics/src/main.rs`
- `cmd_run` at lines 1144-1200 reads workflow YAML, compiles with `vb_compile::compile_workflow`, maps inputs, calls `store_workflow_artifacts` for durable modes, then calls `run_compiled_workflow`.
- `store_workflow_artifacts` at lines 1203-1246 writes `WorkflowSourceRecord` and `CompiledIrRecord` directly. Evidence: it serializes `compiled.to_parts()` and writes `journal.put_workflow_source` then `journal.put_compiled_ir`; it does not call `vb_storage::admission::submit_artifact*`, so the persisted compiled record is raw `WorkflowParts`, not an `AcceptedArtifact` envelope.
- `cmd_submit` at lines 1256-1399 reads YAML, compiles, writes `WorkflowSourceRecord`, writes `RunHeaderRecord`, and appends `JournalEvent::RunAccepted` for durable modes. Evidence: no accepted artifact write occurs before the `RunAccepted` append.
- `runtime_journal_for_mode` / `open_storage_runtime_journal` at lines 1776-1816 create a storage-backed runtime journal for strict/journaled durability only; they do not pass a storage-backed artifact store to runtime construction.
- `run_compiled_workflow` at lines 1818-1899 constructs `Runtime::new_with_journal(...)` then calls `runtime.submit_compiled_with_inputs(...)`. Current path admits an in-memory `CompiledWorkflow` directly.

### Legacy/non-main CLI helper: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/velvet_ballistics/src/run.rs`
- `cmd_run` at lines 104-139 compiles YAML and directly calls `run_compiled_workflow` with no artifact persistence.
- `cmd_run_compiled` at lines 141-180 accepts raw postcard `WorkflowParts`, validates into `CompiledWorkflow`, and runs. This is valid for relaxed/non-strict artifact testing but is a bypass risk if exposed under strict durability.

### CLI runtime helper: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/velvet_ballistics/src/workflow.rs`
- `run_compiled_workflow` at lines 36-88 creates `Runtime::new_with_journal(...)`; `runtime_journal_for_mode` at lines 90-124 only selects a journal, not an artifact store.

### Runtime construction and admission
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_runtime/src/runtime.rs` lines 47-66: `Runtime::new_with_journal` creates shards using `Shard::new_with_journal(config, journal.clone())`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` lines 32-39: `Shard::new_with_journal` uses `AlwaysPresentArtifactStore::shared()`. This means strict/journaled runtime construction through the current CLI path can pass admission without storage-backed accepted artifact validation.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` lines 155-247: `handle_submit_with_inputs_and_contracts` derives `digest = workflow.digest()` and calls `build_admission`; admission failures happen before run state insertion.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_runtime/src/admission.rs` lines 228-350: `AcceptedArtifactStore` and `StorageArtifactStore` load `vb_storage::admission::AcceptedArtifact` from `compiled_ir`; `StorageArtifactStore` rejects missing record, postcard decode failure, invalid gate count, and false proof flags.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_runtime/src/admission.rs` lines 391-448: `admit_artifact_run` enforces accepted artifact loading for `RuntimePolicy::Strict` and `RuntimePolicy::Journaled`, but only if the shard was constructed with a real `AcceptedArtifactStore`.

### Storage accepted artifact API
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_storage/src/admission.rs` lines 102-115 defines `AcceptedArtifact { digest, ir, verification, accepted_at_seq, required_capabilities }`.
- Same file lines 131-220: `submit_artifact` / `submit_artifact_with_contracts` persist an accepted artifact envelope in `compiled_ir`. Risk: current local constant `ADMISSION_GATE_COUNT` is 2 (line 118), while runtime requires 15 gates (`vb_runtime::admission::REQUIRED_GATE_COUNT`, line 16). This dependency appears intentionally owned by `vb-core-accepted-artifact-format`; downstream contract/proof must resolve before strict CLI acceptance can pass.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_storage/src/artifacts.rs` lines 14-45 lists/removes/checks compiled IR artifacts only by digest.

## Existing tests and evidence points
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/velvet_ballistics/tests/cli_integration.rs` lines 2020-2076: strict CLI run currently asserts run completion and `RunAccepted`/`RunFinished` events. It does not assert accepted artifact envelope persistence or storage-backed admission.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/velvet_ballistics/tests/ir_artifact_admission.rs` lines 85-180: `run-compiled` accepts valid raw handcrafted IR and rejects malformed raw IR under `--durability none`. This is out of strict accepted-artifact admission but must not become a strict bypass.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/velvet_ballistics/tests/admission_evidence_integration.rs` and chunks exist and should be searched by test-planner for admission assertions.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` contains accepted-artifact storage tests and forged source digest tests; useful for storage-side precedent.

## Suspected touched crates
- `velvet_ballistics`: CLI `run`, `submit`, runtime construction, direct `run-compiled` strict behavior, operator output/errors.
- `vb_runtime`: public constructors may need storage-backed accepted artifact store path for CLI strict/journaled runtime admission.
- `vb_storage`: accepted artifact persistence, atomic accepted-run batch, and source/artifact/header/event consistency; likely shared with dependency beads.
- `vb_core`: `RuntimePolicy`, digest types, capability contract surface; no direct schema change unless accepted artifact format dependency lands in this bead.
- `vb_compile` / `vb_yaml`: source compile/validate inputs; likely consumed not edited except tests proving YAML-origin path.

## Public APIs/symbols to preserve or deliberately change
- `velvet_ballistics::main::cmd_run`, `cmd_submit`, `run_compiled_workflow`, `runtime_journal_for_mode` (private but CLI-critical).
- `velvet_ballistics::run::cmd_run`, `cmd_run_compiled` if still wired/exported.
- `vb_runtime::runtime::Runtime::new_with_journal`, possible new constructor equivalent to `new_with_journal_and_artifact_store` for multi-shard runtime.
- `vb_runtime::shard::Shard::new_with_journal_and_artifact_store`, `Shard::new_with_journal`.
- `vb_runtime::admission::{AcceptedArtifactStore, StorageArtifactStore, admit_artifact_run, REQUIRED_GATE_COUNT}`.
- `vb_storage::admission::{AcceptedArtifact, VerificationProof, submit_artifact, submit_artifact_with_contracts}`.
- `vb_storage::{FjallJournal, put_workflow_source, put_run_header, JournalEvent::RunAccepted, CompiledIrRecord, WorkflowSourceRecord}`.

## Contract clauses for downstream State 3
1. Strict YAML `run` must parse/compile from source, persist source and accepted artifact envelope, then runtime-admit by artifact digest loaded from storage before any `RunAccepted`/run-state acknowledgement.
2. Strict `submit` must persist workflow source, accepted artifact, run header, and `RunAccepted` as one fail-closed accepted-run boundary or fail before acknowledgement.
3. Runtime strict/journaled admission must use `StorageArtifactStore` or equivalent storage-backed accepted artifact store; `AlwaysPresentArtifactStore` is test-only or relaxed-only.
4. Raw `WorkflowParts`, raw `CompiledWorkflow`, or loose YAML compilation must not satisfy strict admission unless first converted to and persisted as an accepted artifact with valid proof envelope and digest binding.
5. Digest binding: run header `compiled_digest`, `RunAccepted.workflow`, stored accepted artifact digest, and runtime admission artifact digest must match.
6. Failure cases: missing artifact, malformed accepted artifact envelope, mismatched digest/source, invalid proof flags/gate count, storage write failure, and partial batch failure must reject without durable acknowledgement.

## Risk tags
- `public-api`: CLI behavior and runtime constructors.
- `persistence`: Fjall journal writes and compiled_ir/workflow_source/run_header/event records.
- `temporal`: accepted-run ordering; acknowledge only after durable boundary.
- `parser-codec`: YAML -> compile -> postcard accepted artifact envelope.
- `security`: strict admission bypass through raw compiled input or AlwaysPresentArtifactStore.
- `performance`: strict paths add storage and hashing; benchmark only if claims are made.
- `dependency`: blocked/entangled with accepted artifact format, atomic admission, and storage artifact store beads.
- `release-critical`: this bead is P0 core engine acceptance path.

## Required verifier modes recommended
- `verify-standard` / `moon ci` for canonical gate.
- `unit` and `integration` tests for CLI strict run/submit and strict bypass rejection.
- `proptest` for digest/source/artifact mismatch cases where existing harness supports it.
- `kani` for storage/admission invariants if existing harnesses cover digest binding or batch atomicity.
- `miri` only for storage/codec unsafe/UB regression if existing lane is configured; repository forbids unsafe but persistence codecs are high-risk.
- `tla-plus` or equivalent temporal model if State 3 treats accepted-run atomic boundary as temporal proof obligation.

## Open questions / unknowns
- Exact final accepted artifact v1 gate count is inconsistent in current code (`vb_storage` creates gate_count=2; `vb_runtime` requires 15). Treat as BLOCK_DEPENDENCY until `vb-core-accepted-artifact-format` resolves or contract chooses migration behavior.
- Whether `crates/velvet_ballistics/src/run.rs` is still wired into the binary or is a legacy helper requires call-graph confirmation by implementation agent.
- Whether `cmd_submit` should execute runtime admission immediately or only persist an accepted run for later execution must be fixed in contract.
