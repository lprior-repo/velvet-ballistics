# Contract Specification: vb-qi37.6 Capability Model Enforcement

## Context
- Bead: `vb-qi37.6` - verifier/runtime capability model enforcement.
- Source of truth read for State 3: State 2 `codebase-map.md`, `delivery-scope.jsonl`, `baseline-report.md`, `STATE.md`, and `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.6 --json`.
- Feature contract from bead: action contracts declare required capabilities; verifier rejects missing or excessive grants; accepted artifacts carry capability certificates; runtime dispatch checks typed capabilities; UI consumes the same typed capability data.
- Release stance: Strict/Journaled accepted-artifact paths are release-critical. Relaxed policy may remain a compatibility mode, but it must not be used as release evidence for capability-protected actions.

## Domain Terms
- Capability: typed grant/requirement pair `(name, action_id)` represented by `vb_core::capability::Capability`.
- CapabilitySet: immutable run grant profile represented by `vb_core::capability::CapabilitySet`.
- ActionContract: declared contract for a Do action, including `required_capabilities`.
- AcceptedArtifact: storage-side certificate envelope carrying compiled IR, proof flags/gate count, digest, sequence, and required capabilities.
- RunAdmission: runtime admission record carrying artifact digest, run id, policy, and granted capability profile.
- External action dispatch: any Do action path that can suspend, invoke, or await an external action.

## Assumptions
- Canonical capability match is exact `(name, action_id)` equality. Hierarchical prefixes and partial lexical prefixes are not grants.
- Empty capability names are invalid schema inputs and also grant nothing if a malformed grant reaches `CapabilitySet::grants`.
- Strict/Journaled release admission requires the runtime canonical gate count of 15; the mapped storage gate count of 2 is a blocker until aligned by implementation or an approved release waiver.
- Count-exact grants are intentional least-privilege behavior: missing grants and excessive grants both deny admission for capability-protected artifacts.

## Open Questions / Blockers
- REPAIRED_GATE_COUNT_ALIGNMENT: State 3 repair routes INTEG-012 to an executable runtime/storage gate-count command; failure remains State 8 implementation debt, not a blocked contract placeholder.
- REPAIRED_REQUIRED_CAPABILITY_SOURCE: State 3 repair routes INTEG-011 to the exact storage required-capability persistence test command.
- REPAIRED_RUNTIME_GRANT_API: State 3 repair routes INTEG-013 to executable public-grant API presence plus runtime admission exact-profile commands.
- REPAIRED_ACTION_CONTRACT_THREADING: State 3 repair routes INTEG-014 to executable engine/drive contract-threading commands.

## Preconditions
- PRE-001: Every Do action in a verified workflow has exactly one validated `ActionContract` before Strict/Journaled acceptance.
- PRE-002: Every required capability has a non-empty schema-valid name and an `action_id` equal to the owning action contract id.
- PRE-003: Duplicate required capabilities within one action contract are rejected before artifact acceptance.
- PRE-004: Accepted artifacts for Strict/Journaled admission carry the complete required-capability profile derived from validated action contracts.
- PRE-005: Accepted artifacts for Strict/Journaled admission carry the canonical 15-gate proof certificate or fail admission with a typed gate-count/proof-flag error.
- PRE-006: Run admission receives an explicit, immutable `CapabilitySet` from the caller/profile before allocating runnable state.
- PRE-007: Engine Do execution receives the validated action-contract set and the admitted run capability profile.

## Postconditions
- POST-001: Validation rejects missing action contracts, unused contracts, invalid capability names, action mismatches, and duplicate capability requirements with exact typed diagnostics.
- POST-002: Artifact acceptance preserves the required-capability profile exactly; non-empty requirements are never erased to an empty certificate.
- POST-003: Strict/Journaled admission accepts only if artifact proof gates are valid, grant count equals required count, and each required capability has an exact grant.
- POST-004: Missing grants, excess grants, action mismatches, prefix grants, partial-prefix grants, legacy bypass attempts, malformed envelopes, and bad proof gates deny admission with typed errors.
- POST-005: Admission denial allocates no run frame, creates no runnable state, and journals no successful `RunAdmission`/`RunAccepted` event.
- POST-006: Do execution without a resolved contract denies with a typed capability/contract-required error before external dispatch.
- POST-007: Do execution with a resolved contract checks every required capability before suspension, await, or external side effect.
- POST-008: UI action registry data is a projection of the same typed capability requirements used by validation, storage, and runtime; UI display cannot be a separate authority.

## Invariants
- INV-001: Capability identity is exact `(name, action_id)` equality; no hierarchical, lexical-prefix, wildcard, empty-name, or action-mismatched grant can satisfy a requirement.
- INV-002: No capability amplification: compile, validation, accepted artifact persistence, admission, runtime dispatch, CLI, and UI must never add capabilities that were not present in validated action contracts and the explicit run grant profile.
- INV-003: For Strict/Journaled release admission, accepted-artifact proof gate count is canonical 15 and all required proof flags are true.
- INV-004: Capability profile cardinality is exact at admission: `granted.len() == required.len()` and every required capability is exactly granted.
- INV-005: Fail-closed lifecycle: any capability, contract, proof, or envelope denial leaves the run non-runnable and non-journaled as accepted.
- INV-006: External Do dispatch is reachable only after admission success, action-contract resolution, taint checks for deterministic-pure actions, and capability checks.
- INV-007: Capability-denial diagnostics retain action id, required capability, and granted profile sufficient for operator evidence without granting side effects.

## Error Taxonomy
- `ValidationError::CapabilityNameEmpty` - required capability name is empty.
- `ValidationError::CapabilityNameTooLong` - capability name exceeds the schema byte limit.
- `ValidationError::CapabilityNameInvalid` - capability name violates lowercase dot-segment grammar.
- `ValidationError::CapabilityActionMismatch` - capability action id differs from the owning action contract.
- `ValidationError::CapabilityDuplicate` - an action contract repeats the same required capability.
- `AdmissionError::CapabilityDenied` - required capability is missing, mismatched, prefix-only, or grant profile is not exact.
- `AdmissionError::ArtifactInvalidGateCount` - accepted artifact proof gate count differs from 15.
- `AdmissionError::ArtifactInvalidProofFlag` - required proof flag is false.
- `AdmissionError::ArtifactEnvelopeDecodeFailed` / `ArtifactEnvelopeError::*` - accepted artifact is absent or malformed.
- `RuntimeError::AdmissionCapabilityDenied` - shard/runtime submit maps admission denial to public runtime error.
- `EngineError::CapabilityDenied` - Do execution denies before external dispatch.

## Contract Signatures
- `fn validate_action_contract_capability_schema(contract: &ActionContract, workflow: &WorkflowParts) -> Result<(), ValidationError>`
- `fn submit_artifact(..., contracts: &[ActionContract], policy: RuntimePolicy) -> Result<AcceptedArtifact, StorageError>`
- `fn admit_artifact_run(store: &dyn AcceptedArtifactStore, policy: RuntimePolicy, run: RunId, digest: WorkflowDigest, caps: CapabilitySet) -> Result<RunAdmission, AdmissionError>`
- `fn check_capability(action: ActionId, required: &Capability, granted: &CapabilitySet) -> Result<(), AdmissionError>`
- `fn execute_do(node: ..., contracts: &[ActionContract], granted: &CapabilitySet) -> Result<ActionOutcome, EngineError>`
- `fn render_action_registry(actions: &[ActionDescriptionView]) -> Result<SystemStatusView, UiModelError>`

## Verus-Owned Clauses
- INV-001, INV-002, INV-004, POST-002, and PRE-002/PRE-003 schema abstractions are Rust-local pure/core obligations.
- Existing model target: `verification/verus/capability_artifact_model.rs`.

## TLA+-Owned Clauses
- POST-003, POST-004, POST-005, POST-006, POST-007, INV-003, INV-005, INV-006 are lifecycle/state-over-time obligations.
- Existing model target: `verification/tla/CapabilityLifecycle.tla` with `CapabilityLifecycleAll.cfg`.

## Theorem-Owned Clauses
- None for State 3. Verus owns the tiny algebraic kernel for exact matching, profile cardinality, and certificate preservation.

## Non-goals
- No UI implementation, Makepad layout, production code, test code, or proof-code changes in State 3.
- No generated Rust/maxperf/codegen acceptance in this engine-only capability contract.
