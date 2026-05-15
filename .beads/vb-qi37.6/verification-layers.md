# Verification Layers: vb-qi37.6

## Boundary
- Verus-owned kernel: exact match, profile cardinality, schema abstraction, certificate preservation.
- TLA+ temporal model: Strict/Journaled admission/dispatch lifecycle.
- Runtime shell: storage, admission APIs, shard, engine execution, UI projection.
- External systems excluded from formal proof: Fjall I/O, postcard bytes, CLI shell, Makepad/UI rendering, generated Rust.

## Layer Assignment
- PRE-001 -> unit/integration validation tests + mutation.
- PRE-002/PRE-003 -> Verus abstraction + fuzz/proptest + unit diagnostics.
- PRE-004/POST-002 -> Verus certificate model + storage integration + serde roundtrip.
- PRE-005/INV-003 -> TLA+ gate mismatch model + integration tests for 15-gate strict artifact acceptance/rejection.
- PRE-006/POST-003/POST-004/INV-004 -> Kani + TLA+ + runtime integration tests.
- PRE-007/POST-006/POST-007/INV-006 -> TLA+ no-contract model + engine/shard integration tests.
- POST-005/INV-005 -> TLA+ + journal/state integration tests.
- POST-008/INV-002 -> UI model serde/unit tests and traceability review.
- ERR-* -> Fowler scenarios + mutation tests.
- Release-critical bundle -> `moon ci` plus scoped formal verifier execution.

## Verus Scope
- Rust target: `verification/verus/capability_artifact_model.rs`.
- Spec/proof functions: `exact_capability_match`, `exact_profile`, `accepted_certificate_preserves_profile`, `gate12_schema_valid`, and proof fns listed in `lean-contract.md`.
- Invariants: exact identity, cardinality equality, non-empty contract not erased, invalid schema rejected.
- Trusted boundary: validated `ActionContract` inputs, concrete string grammar parser, concrete `CapabilitySet` iteration, storage byte encoding.
- Shell exclusions: I/O, async/scheduler behavior, Fjall, postcard, UI, CLI, wall-clock time.
- Evidence command: `verus verification/verus/capability_artifact_model.rs`.

## TLA+ Scope
- Module/model path: `verification/tla/CapabilityLifecycle.tla`.
- Config: `verification/tla/CapabilityLifecycleAll.cfg` plus focused cfg files.
- Variables/actions/invariants: listed in `tla-spec.md`.
- Temporal properties: safety-only; liveness waived for this bead.
- Fairness/deadlock stance: no fairness, no deadlock claim, stutter allowed.
- Refinement boundary: Strict/Journaled admission and Do drive states refine TLA actions.
- Evidence command: `tlc -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla`.

## Kani Scope
- `crates/vb_core/src/kani_capability_harnesses.rs`: exact match and prefix rejection.
- `crates/vb_runtime/src/kani_capability_harnesses.rs`: `check_capability` Ok/CapabilityDenied behavior.
- Evidence commands:
  - `cargo kani -p vb_core --harness capability_name_grants_harness`
  - `cargo kani -p vb_runtime --harness check_capability_harness`

## Fuzz/Property Scope
- `fuzz/src/bin/capability_name_schema.rs` and `fuzz/src/bin/capability_contract_schema.rs` exercise schema validation.
- Evidence commands:
  - `cargo fuzz run capability_name_schema -- -runs=1000`
  - `cargo fuzz run capability_contract_schema -- -runs=1000`

## Integration/CI Scope
- Executable integration obligations replacing State 6 placeholders:
  - INTEG-011: `TMPDIR=.tmp RUSTC_WRAPPER= cargo test -p vb_storage submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability --lib`.
  - INTEG-012: `TMPDIR=.tmp RUSTC_WRAPPER= sh -c 'cargo test -p vb_runtime admit_artifact_run --lib && rg -n "REQUIRED_GATE_COUNT: u8 = 15|gate_count: REQUIRED_GATE_COUNT|ArtifactInvalidGateCount" crates/vb_runtime/src/admission.rs && rg -n "ADMISSION_GATE_COUNT: u8 = 15|gate_count: ADMISSION_GATE_COUNT" crates/vb_storage/src/admission.rs'`.
  - INTEG-013: `TMPDIR=.tmp RUSTC_WRAPPER= sh -c 'cargo test -p vb_runtime admit_artifact_run_rejects_missing_grants_without_allocation --lib && cargo test -p vb_runtime admit_artifact_run_rejects_excess_grants --lib && cargo test -p vb_runtime admit_artifact_run_preserves_non_empty_required_capabilities --lib && rg -n "submit_direct_with_grants|submit_compiled_with_grants|submit_direct_with_inputs_grants_and_contracts|SubmitWithContracts" crates/vb_runtime/src/runtime.rs crates/vb_runtime/src/shard/types.rs'`.
  - INTEG-014: `TMPDIR=.tmp RUSTC_WRAPPER= sh -c 'cargo test -p vb_runtime execute_do_succeeds_when_required_capability_is_granted --lib && cargo test -p vb_runtime execute_do_returns_capability_denied_when_required_capability_not_granted --lib && cargo test -p vb_runtime cat10_do_awaiting_action --lib && cargo test -p vb_runtime cat10_do_without_contract_rejects --lib && rg -n "&state.action_contracts|action_contracts:" crates/vb_runtime/src/shard/lifecycle/chunk_002.rs crates/vb_runtime/src/shard/types.rs'`.
- Package tests for mapped crates: `cargo test -p vb_core -p vb_validate -p vb_storage -p vb_runtime -p velvet-ballastics-workspace`.
- Canonical repo gate: `moon ci`.
- Baseline caveat from State 2: repo-wide format may hit pre-existing fuzz parse debt; formal verifier must classify unrelated pre-existing workspace debt as `DEFERRED_GLOBAL`, not bead-local failure.

## Waivers / Non-goals
- Lean: waived; Verus owns the small theorem kernel.
- Liveness: waived for this bead; whole-workflow scheduler liveness belongs to broader runtime beads.
- INTEG-011..INTEG-014: no waiver used after repair; each now has an executable command in `proof-obligations.jsonl`.
- Performance/assembly/API/release-provenance: no new speed, zero-cost, public semver, or release SBOM claim in this contract beyond normal release bundle gates.
