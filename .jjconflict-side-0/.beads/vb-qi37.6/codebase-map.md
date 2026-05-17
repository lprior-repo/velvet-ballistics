# vb-qi37.6 State 2 Codebase Map

bead_id: vb-qi37.6
scope: verifier/runtime capability model enforcement
workspace: /home/lewis/src/vb-qi37-6

## Path Guard

- `pwd -P` in the isolated workspace returned `/home/lewis/src/vb-qi37-6`.
- Forbidden source checkout `/home/lewis/src/Velvet-ballistics` was not used for code or artifact reads/writes.
- State 2 artifacts are written only under `/home/lewis/src/vb-qi37-6/.beads/vb-qi37.6/`.

## Explore Skill Citation

- Read `/home/lewis/.agents/skills/explore/SKILL.md`.
- Relevant cited rules: write only `.beads/<bead-id>/codebase-map.md` and `.beads/<bead-id>/delivery-scope.jsonl`; every map entry must be backed by a path, symbol, command result, or explicit unknown marker; verify non-empty artifacts and JSONL parsing before finalizing.

## State Inputs Read

- `.beads/vb-qi37.6/STATE.md`: current_state 1, status `BLOCK_LOCAL`, owner_state 2, rerun_from 2. It records prior focused repair edits in `crates/vb_core/src/capability.rs`, `crates/vb_core/src/kani_capability_harnesses.rs`, `crates/vb_runtime/src/admission.rs`, `crates/vb_runtime/src/engine/action.rs`, `crates/vb_runtime/src/engine/drive.rs`, `crates/vb_runtime/src/engine/execute.rs`, `crates/vb_runtime/src/engine/tests.rs`, `crates/vb_runtime/src/kani_capability_harnesses.rs`, and `crates/velvet_ballastics/tests/admission_evidence_integration/chunk_003.rs`.
- `.beads/vb-qi37.6/baseline-report.md`: baseline path `/home/lewis/src/vb-qi37-6`, HEAD `c6272854a341ff3e5017db2aae703aa6d1483d7f`, repo-wide `cargo fmt --check` caveat includes pre-existing `fuzz/src/bin/step_budget_new.rs:2:1 expected item, found '!'`.

## Capability Model Core

- `crates/vb_core/src/capability.rs`
- APIs: `Capability::new`, `Capability::name`, `Capability::action_id`, `CapabilitySet::empty`, `CapabilitySet::from_grants`, `CapabilitySet::grants`, `CapabilitySet::len`, `CapabilitySet::is_empty`.
- Current behavior: `CapabilitySet::grants` iterates grants, ignores empty grant names, requires `capability_name_exact(grant.name(), required.name())`, and requires `grant.action == required.action`.
- Tests present in same file: exact grant accepts, hierarchical prefix rejects, short lexical prefix rejects, sibling prefix rejects, action mismatch rejects, empty name grants nothing.
- Risk tags: auth/security, public API, parser/codec. Exact-match semantics must remain stable because runtime admission and engine action checks depend on it.

## Runtime Admission

- `crates/vb_runtime/src/admission.rs`
- APIs and types: `RunAdmission`, `AdmissionError`, `ArtifactStore`, `AcceptedArtifactStore`, `StorageArtifactStore`, `admit_run`, `admit_artifact_run`, `admit_run_with_budget`, `check_capability`.
- `admit_artifact_run` loads accepted artifacts under `Strict` and `Journaled`, maps envelope errors, checks `caps.len() == artifact.required_capabilities.len()`, then checks each required capability with `check_capability`.
- `check_capability` returns `Ok(())` only if `CapabilitySet::grants` succeeds; otherwise returns `AdmissionError::CapabilityDenied` carrying action, required, and granted values.
- Tests present: direct capability grant/deny, hierarchical grant rejection, partial-prefix grant rejection, `admit_artifact_run_rejects_excess_grants`, and `admit_artifact_run_preserves_non_empty_required_capabilities`.
- Current risk: count-exact admission rejects extra grants. This is strict least-privilege behavior if intended, but it makes grant-set cardinality part of the contract and should be explicitly covered by downstream contract/proof/test artifacts.
- Current risk: `REQUIRED_GATE_COUNT` in runtime admission is 15, while storage `submit_artifact` produces accepted artifacts with gate count 2 for `Journaled` and `Strict`. If the same serialized `AcceptedArtifact` path is used end-to-end under strict policies, admission can reject storage-produced artifacts unless another layer upgrades the gate count.

## Storage Admission

- `crates/vb_storage/src/admission.rs`
- APIs and types: `VerificationWarning`, `ProofFlag`, `VerificationProof`, `AcceptedArtifact`, `submit_artifact`, `admit_compiled_artifact`.
- `AcceptedArtifact` includes `required_capabilities: Box<[vb_core::capability::Capability]>`.
- `submit_artifact` currently persists `required_capabilities: Box::new([])` in Relaxed, Journaled, and Strict paths.
- Storage tests cover relaxed persistence, journaled gate count 2, strict durable flag, checksum validation, serde roundtrip, and warning gate boundary values.
- Current risk: storage admission does not derive required capabilities from action contracts or compiled workflow metadata in this file. Runtime admission can enforce non-empty artifact capability requirements only when a store returns an `AcceptedArtifact` with `required_capabilities` populated by another path.
- Current risk: `VerificationWarning::MAX_GATE` is 2 by storage contract comment, while runtime admission requires 15 accepted gates. This needs contract alignment before release-critical strict admission evidence.

## Engine Action Enforcement

- `crates/vb_runtime/src/engine/action.rs`
- APIs: `execute_do`, `execute_do_without_contract`, `execute_retry_check`, `execute_error_handler`, `resume_action_outcome`, `compute_idempotency_key`, `resolve_contract`.
- `execute_do` resolves the runtime action contract by `ActionId`, rejects unclean taint for deterministic-pure actions, checks every `resolved.required_capabilities` with `check_capability`, and maps denied capability to `EngineError::CapabilityDenied`.
- `execute_do_without_contract` conservatively reads input taint, rejects non-clean input, then always returns `EngineError::CapabilityDenied` with required capability `__contract_required__` for the action.
- Tests present around no-contract rejection, known-contract action suspension, idempotency key determinism, retry behavior, and error handler behavior.
- Risk tags: auth/security, temporal, user-visible behavior. The no-contract path blocks Do execution rather than bypassing capabilities; this is safe-by-default but can break workflows unless contracts are consistently threaded into runtime execution.

## Engine Dispatch and Shard Drive

- `crates/vb_runtime/src/engine/execute.rs`
- `execute_node_full` receives `contracts: &[ActionContract]` and `granted: &CapabilitySet`; Do nodes route to `execute_do_without_contract` when `contracts.is_empty()` and to `execute_do` otherwise.
- `crates/vb_runtime/src/engine/drive.rs`
- `drive_deterministic_full` receives `contracts` and `granted`, then forwards both to `execute_node_full` for every driven step.
- `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`
- `Shard::drive_state` builds `granted` from `state.admission.as_ref().map(|a| a.granted_capabilities()).unwrap_or(&CapabilitySet::empty())`, but passes `&[]` for action contracts into `drive_deterministic_full`.
- Current risk: shard drive currently never provides action contracts to the engine; any Do node goes through `execute_do_without_contract` and returns capability denied. This prevents runtime capability bypass but also prevents normal contracted Do execution through shard-owned runs.

## Runtime Public API and Shard Admission

- `crates/vb_runtime/src/runtime.rs`
- APIs: `Runtime::submit_direct`, `Runtime::submit_compiled`, `Runtime::submit_compiled_with_inputs`, `Runtime::tick_all`, action completion/failure APIs.
- `submit_direct`, `submit_compiled`, and `submit_compiled_with_inputs` all enqueue shard submit commands with `CapabilitySet::empty()`.
- `crates/vb_runtime/src/shard/types.rs`
- `ShardCommand::{Submit, SubmitPrePersisted, SubmitWithInputs}` all carry `caps: CapabilitySet`.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
- `handle_submit*` calls `build_admission`, which calls `admit_artifact_run(self.artifact_store.as_ref(), self.policy, run, digest, caps)` and maps `CapabilityDenied` to `RuntimeError::AdmissionCapabilityDenied`.
- Current risk: public `Runtime` submit APIs do not expose a way to provide non-empty run capabilities. Direct `ShardCommand` can carry caps, but `Runtime` callers cannot grant required capabilities through the public API shown.

## UI Model Surface

- `crates/vb_ui_model/src/system.rs`
- `ActionDescriptionView` includes `required_capabilities: Box<[Capability]>` alongside action id, side effect, idempotency, retry safety, timeout, and IO bounds.
- Risk tags: public API, user-visible behavior. UI model can display capability requirements, but enforcement source of truth remains core/storage/runtime.

## Validation Gate 12 Capability Schema

- `crates/vb_validate/src/gates.rs`
- APIs: `validate_gate_12_action_contract_completeness`, `validate_action_contract_capability_schema`, `validate_required_capability`, `validate_capability_name`, `validate_no_duplicate_capability_requirements`.
- Capability schema enforced by Gate 12: non-empty name, byte length <= `MAX_CAPABILITY_NAME_BYTES`, grammar is lowercase/digit/underscore dot-separated segments with no empty segment, capability action id must match contract action id, duplicate capability requirements in a contract are rejected.
- `crates/vb_validate/src/lib.rs` defines diagnostics `CapabilityNameEmpty`, `CapabilityNameTooLong`, `CapabilityNameInvalid`, `CapabilityActionMismatch`, and `CapabilityDuplicate`.
- `tests/gate_12_14_15_tests.rs` includes `gate_12_contract_capability_validation`, but only checks empty capability name via `result.is_err()`; stronger tests should assert exact diagnostic variants for mismatch, duplicates, invalid grammar, and too-long names.

## Existing Tests and Evidence Hooks

- `tests/gate_12_14_15_tests.rs`: Gate 12 action contract completeness and basic capability validation, Gate 14 slot typing, Gate 15 determinism.
- `crates/vb_core/src/capability.rs`: unit tests for exact matching and prefix rejection.
- `crates/vb_runtime/src/admission.rs`: unit tests for admission and capability denial.
- `crates/vb_runtime/src/engine/action.rs`: unit tests for no-contract rejection and action behavior.
- `crates/vb_runtime/src/engine/execute.rs`: tests cover Do dispatch rejection when contracts are missing.
- `crates/vb_runtime/src/engine/drive.rs`: helpers and tests include `CapabilitySet` and direct drive with grants.
- `crates/velvet_ballastics/tests/admission_evidence_integration/chunk_001.rs` and `chunk_002.rs`: integration coverage for `submit_artifact` persistence and runtime completion under relaxed policy, but the visible tests use workflows without action capability requirements.
- `fuzz/src/lib.rs`: `fuzz_capability_name_schema` and `fuzz_capability_contract_schema` exercise capability schema validation through `vb_validate::shared::validate_with_contracts`.
- Fuzz target binaries: `fuzz/src/bin/capability_name_schema.rs`, `fuzz/src/bin/capability_contract_schema.rs`.

## Existing Formal/Verification Artifacts

- `crates/vb_core/src/kani_capability_harnesses.rs`: Kani harnesses for exact match, prefix-dot rejection, partial segment rejection, non-prefix rejection, and panic-free/deterministic grant checks over bounded names.
- `crates/vb_runtime/src/kani_capability_harnesses.rs`: Kani harnesses for `check_capability`, action match/name grants, action/name denies, hierarchical subpath rejection, and partial segment rejection.
- `verification/verus/*`: no capability-specific Verus files found. Existing Verus files are value store, taint lattice, step state machine, step budget, signals, run loop termination, resource budget, diagnostic envelope, and budget proofs.
- `verification/tla/**/*`: no files found by focused glob.
- `**/*capab*.tla`: no files found.

## Blockers and Open Questions

- `BLOCKER_GATE_COUNT_ALIGNMENT`: `vb_runtime::admission::REQUIRED_GATE_COUNT == 15`, while `vb_storage::admission::submit_artifact` writes `VerificationProof::new(..., ADMISSION_GATE_COUNT, durable)` with `ADMISSION_GATE_COUNT == 2`. Downstream contract/proof work must decide the canonical accepted-artifact gate count before strict/journaled release evidence.
- `BLOCKER_REQUIRED_CAPABILITY_SOURCE`: `submit_artifact` writes empty `required_capabilities`; the map found no storage-side derivation from `ActionContract` in the mandatory storage file. Downstream design must identify where required capabilities enter accepted artifacts.
- `BLOCKER_RUNTIME_GRANT_API`: public runtime submit APIs enqueue empty `CapabilitySet`; no public runtime method in `runtime.rs` accepts caller grants. If required capabilities are non-empty, public runtime callers cannot satisfy strict admission through the mapped API.
- `BLOCKER_ACTION_CONTRACT_THREADING`: shard `drive_state` passes empty `contracts` to `drive_deterministic_full`, causing Do nodes to use `execute_do_without_contract` and deny by construction. Downstream implementation must thread validated action contracts into runtime state or explicitly document Do execution as unsupported in this path.
- `UNKNOWN_CAPABILITY_TLA`: no TLA capability spec was found. If temporal properties are required for admission/run sequencing, new TLA artifacts are needed.
- `UNKNOWN_CAPABILITY_VERUS`: no capability-specific Verus proof was found. Existing Kani harnesses cover bounded matching and `check_capability`, but not end-to-end storage-to-runtime admission.

## Recommended Downstream Owners

- `rust-contract`: lock the canonical capability contract, including exact-match semantics, grant cardinality, gate count, and artifact required-capability source.
- `proof-planner`: require Kani for capability matching/checking, proptest/fuzz for schema grammar, and TLA or state-machine tests for no-run-allocation-on-admission-denial.
- `test-planner`: add integration tests for non-empty accepted artifact requirements, extra grants, missing grants, public runtime grant flow, and action contract threading.
- `holzman-rust`: enforce zero-panic bounded loops and no unsafe regressions while fixing API/threading gaps.
