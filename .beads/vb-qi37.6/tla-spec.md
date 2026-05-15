# TLA+ Temporal Model Plan: CapabilityLifecycle

## Boundary
- Temporal behavior: Strict/Journaled capability admission and Do execution lifecycle from pending admission through deny/admit, run allocation, journaling, and drive-state outcome.
- Rust/core behavior excluded and handled by Verus/Kani/tests: exact byte/name equality, action id equality, schema grammar, concrete `CapabilitySet` representation, postcard/Fjall encoding.
- External systems abstracted: storage lookup, verifier gate production, runtime journal, shard scheduler, and external action executor are abstract booleans/counts.

## TLA+-Owned Clauses
- POST-003: Strict/Journaled admission accepts only 15-gate exact capability profiles.
- POST-004: missing/excess grants, legacy bypass, and gate mismatch deny.
- POST-005 / INV-005: denied admission allocates no run and journals no accepted event.
- POST-006: Do without contracts denies before await/dispatch.
- POST-007 / INV-006: contracted Do can await external action only after exact profile and contracts are present.
- INV-003: canonical gate count is 15.

## Existing Model Shape
- Module/model path: `verification/tla/CapabilityLifecycle.tla`.
- Configs: `verification/tla/CapabilityLifecycleAll.cfg`, `CapabilityLifecycleExactProfile.cfg`, `CapabilityLifecycleExcessGrant.cfg`, `CapabilityLifecycleGateMismatch.cfg`, `CapabilityLifecycleLegacyBypass.cfg`, `CapabilityLifecycleNoContract.cfg`.
- Variables: `gate_count`, `required_count`, `grant_count`, `contracts_present`, `legacy_path`, `admission`, `run_allocated`, `journaled`, `drive_state`.
- Init action: `Init`.
- Next/actions: `DenyGateMismatch`, `DenyCapabilityProfile`, `DenyLegacyBypass`, `AcceptAdmission`, `DriveDoWithoutContracts`, `DriveDoWithContracts`, `Stutter`, `Next`.
- State constraints: finite gate counts `{0, 2, CanonicalGate}`, capability counts `0..2`, boolean flags, finite admission/drive strings.
- Symmetry sets: none needed for the current count abstraction.
- Bounded model limits: count abstraction covers zero, one, and two requirements/grants; concrete capability identity remains Verus/Kani-owned.

## Properties
- Safety invariants: `ExactProfileRequired`, `ExcessGrantDenied`, `NoAdmissionOnGateMismatch`, `NoRunAllocatedOnDeniedAdmission`, `NoDoAwaitingWithoutContract`, `ContractedDoRequiresExactGrant`, `NoLegacyBypassForProtectedSubmit`.
- Liveness/eventuality: not required at State 3 because the model is fail-closed safety for single-step admission/dispatch; eventual scheduler progress belongs to whole-workflow runtime beads.
- Fairness assumptions: none for current safety-only finite lifecycle model; `CHECK_DEADLOCK FALSE` is explicit because stuttering is allowed and liveness is out of scope.
- Deadlock freedom: not claimed by this model.
- Refinement to Rust/runtime behavior: runtime Strict/Journaled `admit_artifact_run` refines pending->denied/admitted transitions; successful admission refines `RunAdmission` allocation and journal acceptance; shard `drive_state` and engine Do execution refine contract-present/absent drive transitions.

## Evidence Commands
- `tlc -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla`
- `tlc -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla`
- `tlc -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla`
- `tlc -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla`
- `tlc -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla`
- `tlc -config verification/tla/CapabilityLifecycleNoContract.cfg verification/tla/CapabilityLifecycle.tla`

## Waivers
- Liveness waiver: owner `vb-qi37.6`, reason `capability admission is a fail-closed safety gate; scheduler/run-loop progress is outside this bead`, expiry `before vb-engine-yaml release gate`, compensating evidence `integration/Bdd state-machine tests for denied and admitted paths plus whole-workflow runtime liveness beads`.
