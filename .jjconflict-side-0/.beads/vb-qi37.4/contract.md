# Contract Specification: vb-qi37.4

## Context
- Feature: accepted-artifact admission and run-header persistence for strict/journaled runtime run creation.
- Source bead: `vb-qi37.4`, `runtime/storage: Prove accepted-artifact admission and run-header persistence`.
- State 2 artifacts read: `baseline-report.md`, `codebase-map.md`, `delivery-scope.jsonl`, `STATE.md`.
- Authoritative bead JSON read with: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.4 --json`.

## Domain Terms
- Accepted artifact: postcard-encoded `vb_storage::admission::AcceptedArtifact` stored by digest in `CompiledIrRecord`.
- Verification proof: `vb_storage::admission::VerificationProof`, including gate count and required proof flags.
- Admission: `vb_runtime::admission::admit_artifact_run` plus `RunAdmission` metadata attached before run state insertion.
- Header-before-ack: durable `RunSubmitted`/`RunAdmission` persistence must succeed before the caller may observe successful run creation.
- Strict durability: `FjallJournal::append_strict` or equivalent path reaches `persist_strict()` / `fjall::PersistMode::SyncAll` before success.

## Assumptions
- State 3 repair is contract-only: no production source, tests, proof/model code, or source checkout files are changed.
- Strengthened TLA+ model `specs/admission_header_before_ack.tla` owns persistence-before-ack, live-state-after-persistence ordering, duplicate-run rejection, and no-live-state-on-failure temporal clauses.
- Direct executable proof commands are accepted for State 5 proof evidence while `moon run :verify-proof` is blocked by unrelated canonical wrapper tooling: `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`, `verus verification/verus/admission_artifact_model.rs`, and `verus verification/verus/capability_artifact_model.rs`.
- `vb-qi37.4` is an aggregate feature; open dependent work such as `vb-qi37.4.2`, `vb-core-proof-15-gate`, `vb-core-storage-artifact-store`, and `vb-core-atomic-admission` may still provide production integration evidence.

## Open Questions
- OQ-001: Production shell must prove it extracts `VerificationProof` into the Verus model honestly; the abstract Verus proof fixes the contract-required runtime gate count at `15`.
- OQ-002: Whether `RunHeaderRecord` persistence and `RunAdmission` journal persistence are already a single atomic accepted-run boundary is owned by `vb-core-atomic-admission`.
- OQ-003: Whether strict production paths can still use `AlwaysPresentArtifactStore` is owned by `vb-core-storage-artifact-store`.

## Preconditions
- PRE-001: For `Strict` or `Journaled` run creation, the caller supplies a compiled workflow digest that identifies a stored accepted artifact envelope.
- PRE-002: The accepted artifact envelope must decode as postcard into `vb_storage::admission::AcceptedArtifact`.
- PRE-003: The accepted artifact verification proof must match runtime v1 schema: required gate count is exactly `vb_runtime::admission::REQUIRED_GATE_COUNT` and all required proof flags are true (`bounded`, `taint_safe`, `retry_safe`, `durable`, `replayable`).
- PRE-004: Granted runtime capabilities must exactly cover the accepted artifact required capabilities; missing, excess-count, or non-matching grants are admission failures.
- PRE-005: Strict admission success requires durable persistence of run metadata/admission events before a success acknowledgement is externally observable.
- PRE-006: Duplicate run identifiers are rejected before new runtime state allocation or journal success acknowledgement.

## Postconditions
- POST-001: Successful strict/journaled admission returns or stores `RunAdmission` bound to the requested `RunId`, artifact digest, granted capability set, and runtime policy.
- POST-002: On success, the runtime inserts live run state only after admission succeeds and after required `RunSubmitted`/`RunAdmission` persistence succeeds for strict/header-persisting paths.
- POST-003: On accepted-artifact failure (`missing`, malformed, invalid gate count, false proof flag, capability mismatch), no runnable live state is inserted for the requested run.
- POST-004: On storage failure before header/admission persistence, the operation returns a typed runtime/admission durability error and no success acknowledgement is emitted.
- POST-005: Recovery-visible durable records/events bind run id to the admitted artifact digest, granted capabilities, and runtime policy.

## Invariants
- INV-001: Fail-closed admission: raw, missing, malformed, stale, failed-gate, digest-mismatched, or capability-mismatched artifacts cannot create a strict/journaled runnable run.
- INV-002: Digest binding: accepted artifact digest, compiled IR record digest, run header compiled digest, and `RunAdmission.artifact_digest` must refer to the same workflow digest for a successful run.
- INV-003: Proof-schema binding: strict/journaled runtime accepts only the runtime-required gate count and required true proof flags.
- INV-004: Capability binding: each accepted artifact required capability is granted exactly for the required action/name pair, with no cardinality mismatch hidden as success.
- INV-005: Persistence-before-ack: any storage/admission durability failure before the durable boundary prevents acknowledgement and prevents an externally successful run.
- INV-006: Error fidelity: all admission and durability failures are exposed through typed runtime/API/CLI/IPC diagnostic codes without lossy conversion.
- INV-007: No runtime YAML/JSON/HTTP parsing: strict runtime admission consumes accepted binary artifact/storage records, not source YAML, JSON, or HTTP representations.

## Error Taxonomy
- ERR-001: `AdmissionArtifactNotFound` when no accepted artifact exists for the requested digest.
- ERR-002: `AdmissionArtifactInvalid` when accepted artifact decode, gate count, proof flags, or digest binding fail.
- ERR-003: `AdmissionCapabilityDenied` when required and granted capabilities do not match exactly.
- ERR-004: `RunAlreadyExists` when a duplicate run id is submitted.
- ERR-005: `HeaderPersistenceFailed` or mapped storage/journal error when run header/admission persistence fails before ack.
- ERR-006: `ActiveRunCapacityExceeded` when admission would exceed runtime capacity/budget limits.

## Contract Signatures
- `fn submit_artifact_with_contracts(journal: &vb_storage::FjallJournal, workflow: &vb_core::CompiledWorkflow, policy: vb_core::RuntimePolicy, action_contracts: &[vb_core::action::ActionContract]) -> Result<vb_storage::admission::AcceptedArtifact, vb_storage::JournalError>`
- `fn load_accepted_artifact(store: &dyn vb_runtime::admission::AcceptedArtifactStore, digest: vb_core::WorkflowDigest) -> Result<vb_storage::admission::AcceptedArtifact, vb_runtime::admission::ArtifactEnvelopeError>`
- `fn admit_artifact_run(store: &dyn vb_runtime::admission::AcceptedArtifactStore, policy: vb_core::RuntimePolicy, run_id: vb_core::RunId, artifact_digest: vb_core::WorkflowDigest, caps: vb_core::capability::CapabilitySet) -> Result<vb_runtime::admission::RunAdmission, vb_runtime::admission::AdmissionError>`
- `fn handle_submit_with_inputs_contracts_and_header_mode(...) -> Result<(), vb_runtime::RuntimeError>`
- `fn append_strict(event: &vb_storage::JournalEvent) -> Result<(), vb_storage::JournalError>`
- `fn persist_strict() -> Result<(), vb_storage::JournalError>`

## Verus-Owned Clauses
- PRE-004 / INV-004: exact capability cardinality and per-action match in `verification/verus/capability_artifact_model.rs`, obligation `VERUS-CAP-003`, command `verus verification/verus/capability_artifact_model.rs`.
- PRE-003 / INV-003: proof-schema constants and true-flag validation in `verification/verus/admission_artifact_model.rs`, obligation `VERUS-GATE-004`, command `verus verification/verus/admission_artifact_model.rs`; trusted shell must connect runtime constant and decoded proof fields to `required_gate_count() == 15` and `gate_schema_valid`.
- POST-001 / POST-005 / INV-002: digest equality across accepted artifact/header/admission abstract records in `verification/verus/admission_artifact_model.rs`, obligation `VERUS-DIGEST-005`, command `verus verification/verus/admission_artifact_model.rs`; Fjall lookup, postcard bytes, and digest construction remain shell evidence.

## TLA+-Owned Clauses
- PRE-005 / POST-004 / INV-005: `specs/admission_header_before_ack.tla`, obligation `TLA-ACK-001`, models failure-prevents-ack, ack-requires-persistence, no-live-state-before-durable-admission, and eventual rejection for pending failures.
- PRE-006 / POST-002 / POST-003 / ERR-004: `specs/admission_header_before_ack.tla`, obligation `TLA-STATE-002`, models `duplicate_run` as immediate rejection with no ack/live state and models `live_state` becoming true only in `Ack` after `PersistHeader`.

## Theorem-Owned Clauses
- None for State 3. Verus owns Rust-local pure/model obligations; TLA+ owns lifecycle ordering. Lean/Aeneas/Hax is a non-goal unless proof review finds Verus insufficient for a tiny digest/capability algebra kernel.

## Non-goals
- No production code, tests, proof code, TLA code, or Verus code in State 3.
- No performance or vectorization claim for this bead beyond no-regression workspace gates.
- No UI behavior requirement; UI consumes typed capability data in dependent capability beads.
