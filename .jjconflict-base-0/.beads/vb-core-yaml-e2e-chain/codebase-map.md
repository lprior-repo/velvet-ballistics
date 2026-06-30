# Codebase map: vb-core-yaml-e2e-chain

State 2 exploration timestamp: 2026-05-15T19:41:46Z  
Isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`  
Source checkout for bd reads only: `/home/lewis/src/velvet-ballistics`

## Bead scope from bd

Bead: `vb-core-yaml-e2e-chain` — `engine: Prove YAML-origin Fjall runtime inspect events recovery chain`.

Acceptance requires executable evidence for:

- strict YAML validate/compile;
- accepted artifact persistence;
- Fjall persistence and strict runtime execution;
- journal/events/inspect/replay/recovery proving digest binding;
- restart/replay recovery without YAML reparsing;
- typed failures for corrupt or mismatched source/artifact digests.

Dependencies that shape scope: `vb-ahfl`, `vb-core-cli-accepted-path`, `vb-core-replay-divergence-recovery`, `vb-core-strict-ack-ordering`, `vb-qi37.1`, `vb-qi37.1.6`, `vb-qi37.4`. Dependent acceptance root: `vb-engine-yaml`.

## Relevant crate clusters

### CLI/end-to-end orchestration: `crates/velvet_ballistics`

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/velvet_ballistics/src/main.rs`
  - `cmd_run` lines 1144-1200 reads YAML, calls `vb_compile::compile_workflow`, stores workflow artifacts for journaled/strict durability, then calls `run_compiled_workflow`.
  - `store_workflow_artifacts` lines 1203-1246 writes `WorkflowSourceRecord` and `CompiledIrRecord` to `vb_storage::FjallJournal`.
  - `cmd_submit` lines 1256-1411 compiles YAML, writes source, run header, and `JournalEvent::RunAccepted` for durable submit.
  - `run_compiled_workflow` lines 1818-1900 submits compiled workflow to runtime and emits completion status.
  - `StorageWorkflowResolver::resolve_workflow` lines 2036-2057 resolves compiled IR from storage by digest and rejects missing/mismatched/invalid artifacts.
  - `cmd_inspect` lines 2060-2131 summarizes status from persisted run events.
  - `cmd_events` / `event_to_json` lines 2133-2429 expose journal events including `RunAccepted` and `RunAdmission` with digests.
- Existing integration evidence:
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/velvet_ballistics/tests/cli_integration.rs` lines 1230-1300 prove journaled YAML run emits `RunAccepted`, `RunFinished`, and inspect status.
  - same file lines 2030-2076 prove strict YAML run completes and emits `RunAccepted`/`RunFinished`.
  - same file lines 2220-2309 constructs persisted journal/snapshot data for doctor/trim checks.

### YAML validation/compile boundary: `crates/vb_compile`, `crates/vb_yaml`, `crates/vb_validate`

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_compile/src/lib.rs`
  - module docs lines 9-13 state YAML enters only through compiler; hot engine consumes `vb_core::CompiledWorkflow`.
  - `YamlCompiler::compile` lines 151-157 parses canonical YAML via `vb_yaml::parse_workflow_source` then compiles source.
  - `YamlCompiler::parse_ast` lines 159-175 performs strict profile, duplicate-key, shape, schema, reference, type/taint, and control-flow validation.
  - public `compile_workflow` starts at line 219 region (searched as the top-level compilation API).
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_compile/src/strict_yaml.rs`
  - lines 1-5 explicitly keep YAML parser dependencies out of runtime crates.
  - `reject_unsupported_profile_events` lines 10-13 and helpers lines 45-88 reject aliases, anchors, tags, and multiple docs.
- Supporting files/globs to inspect when writing tests:
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_yaml/src/**/*.rs`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_validate/src/**/*.rs`

### Accepted artifact and digest-bound storage: `crates/vb_storage`, `crates/vb_runtime`

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/journal/source.rs`
  - `put_workflow_source` lines 14-30 verifies source bytes match claimed digest before storage.
  - `workflow_source`, `put_compiled_ir`, `compiled_ir` lines 32-72 provide digest-keyed source/IR storage.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/journal/admission.rs`
  - `verify_content_digest` lines 3-11 returns `JournalError::PayloadDigestMismatch` on source digest mismatch.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/admission.rs`
  - `AcceptedArtifact` lines 102-115 carries digest, serialized IR, verification proof, accepted sequence, and required capabilities.
  - `submit_artifact` / `submit_artifact_with_contracts` lines 131-223 validate, verify, persist accepted artifact, and call strict persistence for `RuntimePolicy::Strict`.
  - Note risk: storage-side `ADMISSION_GATE_COUNT` is 2 (line 118) while runtime `REQUIRED_GATE_COUNT` is 15; downstream contract/test planning must resolve whether this bead should prove or repair parity.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_runtime/src/admission.rs`
  - `RunAdmission`, `AdmissionError`, `ArtifactStore`, `AcceptedArtifactStore` lines 58-241 define public admission API and errors.
  - `StorageArtifactStore::load_accepted_artifact` lines 304-350 decodes stored artifact and requires gate_count=15 plus proof flags.
  - `admit_run` lines 353-375 performs existence-only admission.
  - `admit_artifact_run` lines 377-448 performs full accepted-artifact validation and capability checks.
- Existing integration evidence:
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/velvet_ballistics/tests/admission_evidence_integration/chunk_002.rs` lines 6-92 proves compile -> `vb_storage::submit_artifact` -> stored artifact -> runtime completion under relaxed policy.

### Fjall journal/events/recovery: `crates/vb_storage`, `crates/vb_runtime`

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/journal/mod.rs` lines 1-17 exposes `FjallJournal` and journal submodules.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/recovery/mod.rs`
  - lines 9-15 document digest mismatch detection, replay divergence, snapshot+tail recovery, and full journal recovery.
  - lines 44-59 re-export recovery/replay/hydrate APIs.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/recovery/recover.rs`
  - `check_workflow_source_digest` lines 21-39 detects `WorkflowSourceDigestMismatch` from `RunAccepted` workflow digest.
  - `check_compiled_ir_digest` lines 42-50 detects `CompiledIrDigestMismatch`.
  - `verify_digests` lines 53-74 combines digest checks.
  - `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_all_incomplete_runs` lines 76-134 recover from persisted events/run headers only.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/recovery/replay/summary.rs`
  - `summarize_recovery_events` lines 88-119 rejects empty or multi-run events.
  - `recover_runtime_frame_seed_from_events_with_workflow` lines 172-180 reconstructs deterministic slot state from durable events and compiled workflow.
  - `reject_workflow_digest_mismatch` lines 182-199 returns `CompiledIrDigestMismatch` on `RunAccepted` digest mismatch.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/recovery/types.rs` lines 16-96 define typed recovery errors: `WorkflowSourceDigestMismatch`, `CompiledIrDigestMismatch`, `ReplayDivergence`, `NoRecoveryData`, `CorruptSnapshot`, `FrameDimensionOverflow`, etc.
- Existing test evidence:
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/src/recovery/tests.rs` contains digest mismatch, replay divergence, no recovery data, snapshot/tail cases (grep hits around lines 154, 166, 332, 895, 1195, 1228, 1450, 2387).
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs` has external recovery contract coverage for event-only frame seed hydration and Fjall journal flush behavior.

## Dependency/config files likely in scope

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/velvet_ballistics/Cargo.toml` depends on `postcard`, `serde-saphyr`, `blake3`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_compile/Cargo.toml` depends on `blake3`, `postcard`, `saphyr`, `saphyr-parser`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_yaml/Cargo.toml` depends on `saphyr`, `saphyr-parser`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_storage/Cargo.toml` depends on `blake3`, `fjall`, `postcard`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain/crates/vb_runtime/Cargo.toml` depends on `postcard`.

No dependency-file change is required by exploration. Downstream implementation should avoid dependency changes unless a missing API is proven.

## Public APIs to preserve/prove

- CLI commands: `run`, `submit`, `inspect`, `events`, `replay`/doctor recovery-adjacent surfaces.
- `vb_compile::compile_workflow`, `YamlCompiler::compile`, `YamlCompiler::parse_ast`.
- `vb_storage::FjallJournal::{put_workflow_source, workflow_source, put_compiled_ir, compiled_ir, events_for_run, append_strict_batch, put_run_header}`.
- `vb_storage::{submit_artifact, submit_artifact_with_contracts, AcceptedArtifact, VerificationProof}`.
- `vb_runtime::admission::{admit_run, admit_artifact_run, StorageArtifactStore, AcceptedArtifactStore, RunAdmission, AdmissionError}`.
- `vb_storage::recovery::{verify_digests, recover_runtime_summary, recover_runtime_frame_seed, recover_runtime_frame_seed_from_events_with_workflow, recover_all_incomplete_runs}`.

## Open questions / risks for downstream states

- `store_workflow_artifacts` stores `CompiledIrRecord` containing raw `WorkflowParts` postcard bytes, while runtime accepted-artifact validation expects `AcceptedArtifact` envelope in `StorageArtifactStore::load_accepted_artifact`. E2E acceptance may require routing YAML strict run/submit through `submit_artifact`/accepted envelope rather than raw compiled IR.
- Runtime accepted-artifact gate count expects 15; storage artifact proof currently uses 2. This is release-critical contract parity risk.
- Recovery functions are storage/event based and do not parse YAML, but E2E evidence must prove the restart path uses stored compiled IR/artifact and never calls YAML parser after initial admission.
- `cmd_run` hardcodes `RunId::new(1)` in `run_compiled_workflow`; tests that run multiple processes against one DB may need isolation or explicit run id handling.
- Source digest and compiled digest are both represented as `WorkflowDigest`; downstream tests must distinguish YAML source digest (`blake3(source)`) from compiled workflow/artifact digest and assert correct typed mismatch errors.

## Recommended verifier/test modes

- Unit/integration: focused tests across CLI `run`/`submit`/`events`/`inspect`, storage admission, and recovery digest mismatch.
- BDD/e2e: Given strict YAML source, When run/submit/restart/replay, Then events/inspect/recovery prove source/artifact digest binding and no YAML reparsing.
- Proptest: corrupt/mismatched digest and journal event sequence variants for recovery fail-closed behavior.
- Kani/Verus: optional/targeted for digest equality and event-sequence recovery invariants if existing harness patterns are reused.
- Miri: storage codec/postcard artifact decode paths if unsafe/UB concerns arise; no unsafe located in scoped modules (`#![forbid(unsafe_code)]` seen in key files).
