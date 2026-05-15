# TLA+ Temporal Model Plan

## Boundary

- Temporal/workflow behavior: strict admission moves from `pending` to `admitted` or `denied`; denied states never allocate a run or emit accepted/runnable state; accepted states require canonical gate count, exact capability profile, and non-legacy path.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests: postcard decoding, digest byte equality implementation, Rust enum mapping, and capability string comparison mechanics.
- External systems abstracted: Fjall storage, journal append durability, CLI/IPC transport, wall-clock staleness source.

## TLA+-Owned Clauses

- PRE-002 / INV-001 -> `verification/tla/CapabilityLifecycle.tla::NoAdmissionOnGateMismatch` with `CapabilityLifecycleGateMismatch.cfg`.
- PRE-005 / INV-006 -> `CapabilityLifecycle.tla::ExcessGrantDenied` and `ExactProfileRequired`.
- PRE-006 / INV-002 -> `CapabilityLifecycle.tla::NoLegacyBypassForProtectedSubmit`.
- POST-003 / INV-005 -> `CapabilityLifecycle.tla::NoRunAllocatedOnDeniedAdmission`.
- POST-001 / POST-005 -> `CapabilityLifecycle.tla::AcceptAdmission` abstraction only; detailed header persistence remains dependent scope.

## Model Shape

- Module/model path: `verification/tla/CapabilityLifecycle.tla`.
- Configs: `verification/tla/CapabilityLifecycleAll.cfg`, `CapabilityLifecycleGateMismatch.cfg`, `CapabilityLifecycleExactProfile.cfg`, `CapabilityLifecycleExcessGrant.cfg`, `CapabilityLifecycleNoContract.cfg`, `CapabilityLifecycleLegacyBypass.cfg`.
- Variables: `gate_count`, `required_count`, `grant_count`, `contracts_present`, `legacy_path`, `admission`, `run_allocated`, `journaled`, `drive_state`.
- Init action: `Init`.
- Next/actions: `DenyGateMismatch`, `DenyCapabilityProfile`, `DenyLegacyBypass`, `AcceptAdmission`, `DriveDoWithoutContracts`, `DriveDoWithContracts`, `Stutter`.
- State constraints: finite gate counts `{0, 2, CanonicalGate}`, finite capability counts `0..2`, booleans for contracts and legacy path.
- Symmetry sets: none required for the existing finite scalar model.
- Bounded model limits: TLC finite model with `CanonicalGate = 15` in `CapabilityLifecycleAll.cfg` and focused configs.

## Properties

- Safety invariants: `ExactProfileRequired`, `ExcessGrantDenied`, `NoAdmissionOnGateMismatch`, `NoRunAllocatedOnDeniedAdmission`, `NoDoAwaitingWithoutContract`, `ContractedDoRequiresExactGrant`, `NoLegacyBypassForProtectedSubmit`.
- Liveness/eventuality: no liveness claim at State 3; admission is a safety gate and TLC configs set `CHECK_DEADLOCK FALSE`.
- Fairness assumptions: none required for safety-only finite admission checks.
- Deadlock freedom: not claimed by existing configs; deadlock check is disabled in `CapabilityLifecycleAll.cfg`.
- Refinement to Rust/runtime behavior: `admission = denied` refines any `AdmissionError` or mapped `RuntimeError` before `take_frame_for`, `self.runs.insert`, `drive_run`, and `RunAccepted`; `legacy_path = TRUE` refines `AlwaysPresentArtifactStore` or existence-only admission paths; `gate_count` refines the accepted envelope gate evidence.

## Evidence Commands

- `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp tlc -metadir .beads/vb-qi37.4.2/tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla`
- `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp tlc -metadir .beads/vb-qi37.4.2/tlc-gate -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla`
- `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp tlc -metadir .beads/vb-qi37.4.2/tlc-exact -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla`
- `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp tlc -metadir .beads/vb-qi37.4.2/tlc-excess -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla`
- `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp tlc -metadir .beads/vb-qi37.4.2/tlc-legacy -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla`

## Waivers

- Temporal staleness expiry is abstracted as an invalid/stale boolean at State 3 because no clock/certificate expiry model path is present. Owner: proof-planner. Expiry: before State 5 proof writing. Compensating evidence: integration tests must inject stale accepted artifacts and assert typed denial before allocation.
