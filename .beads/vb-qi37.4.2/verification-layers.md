# Verification Layers

## Boundary

- Verus-owned kernel: capability exact-cardinality and decoded accepted-envelope gate/durable/status predicates using existing State5 verification artifacts.
- TLA+ temporal model: admission denial and no-allocation safety using `verification/tla/CapabilityLifecycle.tla`.
- Theorem projection: waived; no Lean kernel required at State 3.
- Runtime shell: storage loading, postcard decoding, runtime constructor wiring, CLI/IPC diagnostics.
- External systems excluded from formal proof: Fjall durability internals and wall-clock certificate source.

## Layer Assignment

- PRE-001 -> cargo test integration + fuzz/parser negative inputs + static scan.
- PRE-002 -> TLA+ gate mismatch + Verus accepted-envelope predicate + storage/runtime integration tests.
- PRE-003 -> digest-mismatch integration/domain test; Kani is waived at State3 because no harness exists and no source/proof writes are allowed in this repair.
- PRE-004 -> Verus accepted-envelope predicate + stale/non-durable integration tests.
- PRE-005 -> Verus capability model + TLA+ exact profile + proptest capability profiles.
- PRE-006 -> integration/Bdd constructor path tests + static scan for strict/journaled dummy-store use.
- POST-001 -> integration tests proving no runtime YAML/JSON parse on valid accepted artifact.
- POST-002 -> typed error integration/API/CLI/IPC assertions; mutation is waived at State3 until diagnostic tests exist.
- POST-003 / INV-005 -> TLA+ no allocation on denial + failure-injection integration tests.
- POST-004 / INV-007 -> diagnostics tests for rejected digest and semantic cause.
- INV-001 -> shared gate-count contract proof plus integration assertion storage/runtime agree on `15` or agreed canonical constant.
- INV-002 -> static scan + constructor integration tests.
- INV-003 -> explicit malformed-byte/error scenarios plus decoded Verus predicate; fuzz is waived at State3 because no accepted-envelope fuzz target exists and this repair cannot write tests/proofs.
- INV-004 -> static scan/API integration for accepted-artifact path.
- INV-006 -> Verus + proptest + TLA+.

## Verus Scope

- Existing Rust targets: `verification/verus/capability_artifact_model.rs` and `verification/verus/accepted_envelope_model.rs`.
- Existing spec/proof functions: `exact_capability_match`, `exact_profile`, `accepted_certificate_preserves_profile`, `proof_exact_profile_requires_cardinality`, `proof_missing_or_excess_grants_deny`, `proof_certificate_preserves_required_capabilities`, plus accepted-envelope predicates/proofs for schema support, canonical gate 15, durable flag, non-stale evidence, accepted proof flags, and fail-closed unsupported status denial.
- Trusted boundary: decoded `AcceptedArtifact` values and validated digest/capability inputs.
- Shell exclusions: Fjall I/O, postcard decode implementation, wall-clock source, CLI/IPC transport.

## TLA+ Scope

- Module/model path: `verification/tla/CapabilityLifecycle.tla`.
- Variables: `gate_count`, `required_count`, `grant_count`, `contracts_present`, `legacy_path`, `admission`, `run_allocated`, `journaled`, `drive_state`.
- Actions: `DenyGateMismatch`, `DenyCapabilityProfile`, `DenyLegacyBypass`, `AcceptAdmission`, `DriveDoWithoutContracts`, `DriveDoWithContracts`.
- Safety invariants: `ExactProfileRequired`, `ExcessGrantDenied`, `NoAdmissionOnGateMismatch`, `NoRunAllocatedOnDeniedAdmission`, `NoLegacyBypassForProtectedSubmit`.
- Temporal properties: none claimed; safety-only model.
- Fairness/deadlock stance: no fairness; deadlock check disabled by existing cfg.
- Refinement boundary: runtime admission errors refine denied states; run insertion/drive/accepted events refine allocated/admitted states.
- Evidence command: TLC commands listed in `tla-spec.md`.

## Second-Ring / Runtime Evidence

- Parser/codec: malformed-byte integration/error scenarios must cover raw `WorkflowParts`, YAML/JSON bytes, truncated postcard, wrong schema, and random bytes. Fuzz target is waived until a later state creates `fuzz/fuzz_targets/accepted_artifact_envelope.rs`.
- Kani/proptest: Kani digest harness and proptest feature lane are not execution passes at State3. Contract rows stay `status: planned`; later implementation/formal-verifier must either run exact executable targets when they exist or record WAIVED/DEFERRED results in downstream evidence artifacts.
- Static scan: strict/journaled production paths must not construct `AlwaysPresentArtifactStore`.
- Mutation: not an execution pass at State3. Contract row stays `status: planned`; later test/formal-verifier states must run a bounded cargo-mutants target after diagnostic tests exist or record WAIVED/DEFERRED in downstream evidence artifacts.
- Performance: non-goal. No speed claim is made.
- Release provenance/API compatibility: non-goal for this bead unless public error variants change.

## Waivers

- Lean/Aeneas/Hax waived; see `lean-contract.md`.
- TLA+ liveness waived because admission gate scope is fail-closed safety, not eventual scheduling.
- Kani digest downstream waiver policy: owner `formal-verifier-or-landing`; reason `verification/kani/digest_admission_harness.rs` is absent and State3 repair may not write proof code`; expiry `before formal-verifier can claim Kani digest evidence`; compensating evidence `ERR-005 scenario, integration/domain digest mismatch tests, and TLA+/Verus no-allocation/gate predicate coverage`.
- Fuzz accepted-envelope downstream waiver policy: owner `formal-verifier-or-landing`; reason `fuzz/fuzz_targets/accepted_artifact_envelope.rs` is absent and this repair may not write fuzz/test code`; expiry `before any hostile-byte fuzz claim`; compensating evidence `ERR-002/ERR-003 scenarios and decoded Verus predicate evidence`.
- Proptest invalid-space waiver: owner `rust-contract`; reason `no confirmed proptest feature/target in State3 artifact scope`; expiry `before State8 test implementation if generator target is added`; compensating evidence `explicit Fowler scenarios per invalid class plus exact capability Verus/TLA+`.
- Mutation diagnostic downstream waiver policy: owner `formal-verifier-or-landing`; reason `diagnostic tests do not yet exist and this repair may not write tests`; expiry `after diagnostic tests are implemented or before mutation evidence is claimed`; compensating evidence `ERR-001..ERR-008 exact expected scenarios and traceability rows`.
- Canonical CI downstream deferral policy: owner `formal-verifier-or-landing`; reason `State3 contract repair cannot execute implementation-wide CI as proof approval evidence`; expiry `formal-verifier/landing state`; compensating evidence `JSONL validation and existing State5 TLA+/Verus PASS evidence for proof artifacts`.
