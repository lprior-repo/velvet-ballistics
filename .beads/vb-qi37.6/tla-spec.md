# TLA+ Temporal Model Plan

## Boundary

- Temporal behavior: submit, accepted-artifact lookup, gate validation, capability validation, run allocation, journal admission event, shard drive, Do execution, and denial cleanup.
- Rust-local behavior excluded: byte-level capability exact matching, duplicate detection, and name grammar; handled by Verus/Kani/fuzz/proptest.
- External systems abstracted: Fjall journal as a map from digest to accepted artifact envelope; runtime journal as an append-only sequence; UI as a projection observer.

## TLA+-owned clauses

- INV-003: gate count mismatch `2` vs `15` fails closed.
- INV-005: admission denial is atomic.
- INV-006: missing action contracts cannot produce `AwaitingAction`.
- INV-008: public runtime submit with empty grants is valid only when artifact requirements are empty.
- POST-002, POST-005, POST-006, POST-007, POST-008.

## Model shape

- Planned module path: `verification/tla/capability_admission.tla`.
- Planned config path: `verification/tla/capability_admission.cfg`.
- Variables: `artifacts`, `publicSubmits`, `admissions`, `runs`, `runtimeJournal`, `contractsByDigest`, `driveState`, `uiViews`.
- Init: `Init` creates finite digests/runs/actions with artifacts either absent, gate-count 2, gate-count 15, empty requirements, or non-empty requirements.
- Actions: `SubmitEmptyGrants`, `SubmitWithGrants`, `LoadAcceptedArtifact`, `RejectGateMismatch`, `RejectCapabilityMismatch`, `AcceptAdmission`, `JournalRunAdmission`, `DriveDoWithContracts`, `DriveDoWithoutContracts`, `ProjectUiView`.
- State constraints: finite sets of digests, runs, actions, capabilities, gate counts `{0,2,15}`, and policies `{Relaxed,Journaled,Strict}`.
- Symmetry sets: runs and digests may be symmetric; action IDs are not symmetric when tied to capability pairs.
- Bounded model limits: at least 2 runs, 2 digests, 3 actions, and 3 capabilities so extra-grant, missing-grant, exact-grant, and hierarchical-prefix cases coexist.

## Properties

- `NoAdmissionOnGateMismatch`: under Strict/Journaled, artifact gate count other than `15` never reaches admitted state.
- `NoRunAllocatedOnDeniedAdmission`: denied admission never creates `runs[run]`, never inserts run state, and never appends `RunAdmission`.
- `NoDoAwaitingWithoutContract`: if `contractsByDigest[digest]` is empty and current node is Do, next state is denial, not awaiting action.
- `EmptyPublicGrantRequiresEmptyRequirements`: a public submit with empty grants is accepted only if persisted required-capability set is empty.
- `UiRuntimeParity`: UI projected required capabilities equal the validated action-contract requirements for each action.
- Liveness: if a Strict/Journaled submit has a valid artifact, gate count 15, exact requirements, exact grants, and contracts are threaded, admission eventually reaches either accepted or an explicit downstream engine state.
- Fairness: weak fairness on `LoadAcceptedArtifact`, `Reject*`, and `AcceptAdmission` when enabled.
- Deadlock freedom: every nonterminal submit state has an enabled accept/reject transition.
- Refinement: Rust submit/admission/journal/drive events refine model actions by `run_id`, `digest`, `policy`, required capability set, grant set, and contract presence.

## Evidence command

- Planned after proof-writer creates the model: `tlc -config verification/tla/capability_admission.cfg verification/tla/capability_admission.tla`.
- Until that file exists, State 3 makes no PASS claim.

## Waivers

- None. Temporal coverage is required because admission denial and run allocation are state-over-time security behavior.
