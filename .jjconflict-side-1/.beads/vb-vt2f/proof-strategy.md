# proof-strategy.md - vb-vt2f State 4 proof planning sync

## Scope

- Bead: `vb-vt2f` only.
- State: 4, sublane `proof-plan-sync-after-owner-kani-contract-alignment`, attempt 6 of 7.
- Isolated workdir: `/home/lewis/src/bd-vb-vt2f-bdd`.
- Source checkout (not used for writes): `/home/lewis/src/velvet-ballistics`.
- Planning artifacts only: no proof code, production code, tests, harnesses, or TLA specs were written.

## Objective

Sync State 4 proof-planning artifacts to match the current 40-row State 3 primary ledger after owner-authorized Kani contract alignment. The contract (lines 157-160) explicitly authorizes owner-authorized projection proof kernels as the required Kani proof target, distinguishes them from full concrete-runtime Kani equivalence, and establishes `PROJ-EQ-VT2F-001` as the compensating manual review obligation. The `contract-verification-review.md` (REJECTED) found the prior obligation wording misaligned with the projection-kernel architecture; the contract itself now resolves that conflict. This plan explicitly represents approved Kani projection kernels, the manual projection-equivalence review boundary, commands, expected evidence, assumptions, owner_state, rerun_from, and required/waiver status.

## Ledger Status

- Primary obligation ledger: 40 rows in `.beads/vb-vt2f/proof-obligations.jsonl` (PASS JSONL validation).
- Planned obligation ledger: 40 rows in `.beads/vb-vt2f/proof-obligations.planned.jsonl` (PASS JSONL validation).
- Primary and planned row sets are identical; planned JSONL is the synced output of this state.
- Traceability matrix: 32 rows in `.beads/vb-vt2f/traceability-matrix.jsonl`.

## Required Formal Proof Obligations (owner_state 5)

### TLA-VT2F-LIFECYCLE-001

- **ID**: `TLA-VT2F-LIFECYCLE-001`
- **Contract clauses**: `POST-004,POST-005,POST-006,POST-007,POST-008,POST-009,POST-010,POST-011,INV-001`
- **Target**: `verification/tla/Vt2fRuntimeLifecycle.tla`, `verification/tla/Vt2fRuntimeLifecycle.cfg`
- **Command**: `tlc -config verification/tla/Vt2fRuntimeLifecycle.cfg verification/tla/Vt2fRuntimeLifecycle.tla`
- **Expected evidence**: TLC exits 0; bounded lifecycle invariants (`NoWrongRunMutation`, `TraceListNonDestructive`, `DrainTraceDestructive`, `CancellationRemovesActiveRun`, `ShutdownNoFurtherProgress`, `DeterministicTickOutcome`, `BoundedCountersNoOverflow`) hold; temporal properties (`NoDeadlockWithoutHeartbeatMask`, `EventuallyTerminalOrSuspendedOrTypedErrorWithinBounds`) are not heartbeat-masked; at least two runs and bounded queue/journal/counter/step domains; error transitions modeled.
- **Assumptions**: bounded integer domains; finite run set; weak fairness on enabled Tick/control actions within finite TLC bounds; heartbeat/stutter does not satisfy liveness properties.
- **Mode**: exact-command
- **Owner_state**: 5
- **Rerun_from**: 5
- **Status**: planned
- **Required**: true

### TLA-VT2F-STRICT-ADMISSION-001

- **ID**: `TLA-VT2F-STRICT-ADMISSION-001`
- **Contract clauses**: `POST-012,ERR-002,PRE-005,INV-006`
- **Target**: `verification/tla/Vt2fStrictAdmission.tla`, `verification/tla/Vt2fStrictAdmission.cfg`
- **Command**: `tlc -config verification/tla/Vt2fStrictAdmission.cfg verification/tla/Vt2fStrictAdmission.tla`
- **Expected evidence**: TLC exits 0; no bypass transition admits a missing artifact across bounded policy/store/digest/capability domains; proof distinguishes Missing, AlwaysPresent, and StorageBackedAccepted store modes.
- **Assumptions**: bounded digest/capability sets; finite store modes; explicit shard store construction is distinct from Runtime::new strict missing-store behavior.
- **Mode**: exact-command
- **Owner_state**: 5
- **Rerun_from**: 5
- **Status**: planned
- **Required**: true

### KANI-VT2F-RUNTIME-FACADE-001

- **ID**: `KANI-VT2F-RUNTIME-FACADE-001`
- **Contract clauses**: `POST-008,POST-009,POST-012,ERR-002,ERR-003,ERR-004`
- **Target**: `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs::vt2f_runtime_facade_semantics`
- **Command**: `cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics`
- **Expected evidence**: Kani exits 0 with `VERIFICATION:- SUCCESSFUL`; 0 of 500 failed; 7 of 7 cover properties satisfied; proof-review records no source-level `kani::assume`, stubs, `bounded_any`, or unsafe in the projection kernel.
- **Assumptions**: owner-authorized projection kernel only; matching/stale/absent/wrong-run ticket cover points; missing/accepted/relaxed store cover points; no source-level `kani::assume`, stubs, `bounded_any`, or unsafe.
- **Trusted boundary**: `KernelRuntimeError`, `KernelInspectResponse`, `FacadeKernelState`, `StoreMode`, and `TicketShape` are manual projections of concrete Runtime/shard behavior. Concrete Runtime constructors, store engine, public snapshots/traces, and scheduler shell are excluded from this Kani proof.
- **Limitations**: Does not execute full concrete Runtime public APIs; must not be reused as concrete-runtime Kani equivalence evidence outside vb-vt2f owner-authorized projection-kernel sublane.
- **Expiry**: before any runtime, shard, admission, ask, action failure, journal, trace, or accepted-artifact store-selection semantic edit.
- **Mode**: exact-command
- **Owner_state**: 5
- **Rerun_from**: 5
- **Status**: planned
- **Required**: true

### KANI-VT2F-SHARD-LOWER-001

- **ID**: `KANI-VT2F-SHARD-LOWER-001`
- **Contract clauses**: `POST-008,POST-012,INV-001,INV-003,INV-006,ERR-002,ERR-003`
- **Target**: `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs::vt2f_shard_lower_semantics`
- **Command**: `cargo kani -p vb_runtime --harness vt2f_shard_lower_semantics`
- **Expected evidence**: Kani exits 0 with `VERIFICATION:- SUCCESSFUL`; 0 of 122 failed; 8 of 8 cover properties satisfied; proof-review records no source-level `kani::assume`, stubs, `bounded_any`, or unsafe in the projection kernel.
- **Assumptions**: owner-authorized projection kernel only; Relaxed/Strict/Journaled policy cover points; Missing/AlwaysPresent/StorageBackedAccepted store cover points; bool and non-bool ask prompt cover points; no source-level `kani::assume`, stubs, `bounded_any`, or unsafe.
- **Trusted boundary**: `KernelRuntimeError`, `ShardKernelState`, `StoreMode`, `AskKernelFrame`, and `KernelAskError` are manual projections of concrete lower shard/admission/wait_ask behavior. Concrete shard constructors, admission store engine, and production wait_ask execution are excluded from this Kani proof.
- **Limitations**: Does not execute full concrete shard/admission/ask functions; must not be reused as concrete-runtime Kani equivalence evidence outside vb-vt2f owner-authorized projection-kernel sublane.
- **Expiry**: before any runtime, shard, admission, ask, action failure, journal, trace, or accepted-artifact store-selection semantic edit.
- **Mode**: exact-command
- **Owner_state**: 5
- **Rerun_from**: 5
- **Status**: planned
- **Required**: true

## Required Manual Review Obligations (owner_state 6)

### PROJ-EQ-VT2F-001

- **ID**: `PROJ-EQ-VT2F-001`
- **Contract clauses**: `POST-008,POST-009,POST-012,INV-001,INV-003,INV-006,ERR-002,ERR-003,ERR-004`
- **Target**: `.beads/vb-vt2f/proof-architecture-report.md`, `.beads/vb-vt2f/proof-review.md`, `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs`, `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs`
- **Command**: `review-artifact: contract-verification reviewer must record projection-equivalence mapping, trusted boundaries, limitations, expiry, and non-reuse caveat for KANI-VT2F-RUNTIME-FACADE-001 and KANI-VT2F-SHARD-LOWER-001`
- **Expected evidence**: Reviewer either APPROVES owner-authorized projection equivalence as trusted/manual for vb-vt2f only, or rejects with exact unmapped projection surfaces; approval must state this is not executable equivalence and not reusable as concrete-runtime Kani evidence.
- **Trusted boundary**:
  - `KernelRuntimeError` projects `RuntimeError` variants used by vb-vt2f clauses.
  - `StoreMode` projects missing/always-present/storage-backed accepted artifact store behavior.
  - `FacadeKernelState` and `ShardKernelState` project only queue depth, active/wrong/absent runs, ask slot/taint, and store-policy facts.
  - `TicketShape` projects matching/stale/wrong/absent action and ask ticket classes.
  - `AskKernelFrame` projects bool/non-bool wait_ask validation and executed-count mutation.
- **Limitations**: manual review/waiver only; no executable proof that concrete Runtime/Shard/admission/wait_ask implementations refine the kernels.
- **Expiry**: before any runtime, shard, admission, ask, action failure, journal, trace, or accepted-artifact store-selection semantic edit.
- **Mode**: review-artifact
- **Owner_state**: 6
- **Rerun_from**: 3
- **Status**: planned
- **Required**: true

### WAIVER-VERUS-VT2F-002

- **ID**: `WAIVER-VERUS-VT2F-002`
- **Contract clauses**: `INV-001,INV-002,INV-003,INV-004,INV-005,INV-006`
- **Target**: `.beads/vb-vt2f/proof-plan-review-input.md`
- **Command**: `review-artifact: proof-reviewer must approve or reject WAIVER-VERUS-VT2F-002 after TLA-VT2F-LIFECYCLE-001, TLA-VT2F-STRICT-ADMISSION-001, KANI-VT2F-RUNTIME-FACADE-001, KANI-VT2F-SHARD-LOWER-001, PROJ-EQ-VT2F-001, BDD nextest, catalog nextest, and moon ci evidence are present`
- **Expected evidence**: Explicit State 6 approval or rejection; approval requires passing TLA+ evidence, accepted owner-authorized Kani projection-kernel evidence, explicit projection-equivalence risk acceptance or rejection, BDD nextest, catalog nextest, moon ci or accepted scoped/deferred-global evidence, and finding that non-vacuum Verus binding is infeasible without production refactoring or extraction of a pure transition kernel.
- **Waiver**:
  - **waiver_id**: `WAIVER-VERUS-VT2F-002`
  - **approval_status**: candidate_only
  - **reason**: Direct non-vacuum Verus proof over final mutable runtime/shard implementation is not feasible in this repair without production refactoring; TLA+ covers temporal state space, owner-authorized Kani projection kernels cover bounded bead-local projected Rust semantics, and PROJ-EQ-VT2F-001 exposes the trusted projection risk.
  - **owner**: State 6 proof-reviewer after State 5 proof evidence and State 6 contract-verification projection review.
- **Mode**: review-artifact
- **Owner_state**: 6
- **Rerun_from**: 5
- **Status**: planned
- **Required**: true

## Superseded Waiver Rows (audit only, not approval paths)

| ID | Target | Status | Reason |
|----|--------|--------|--------|
| `WAIVER-TLA-VT2F-001` | runtime lifecycle temporal model | superseded | Replaced by required `TLA-VT2F-LIFECYCLE-001` |
| `WAIVER-TLA-VT2F-002` | strict admission lifecycle temporal model | superseded | Replaced by required `TLA-VT2F-STRICT-ADMISSION-001` |
| `WAIVER-VERUS-VT2F-001` | vb_runtime/vb_core pure transition logic | superseded | Replaced by `WAIVER-VERUS-VT2F-002`, `KANI-VT2F-RUNTIME-FACADE-001`, `KANI-VT2F-SHARD-LOWER-001` |

## Non-Goals

- No proof implementation in this state.
- No production or BDD test edits in this state.
- No TLA/Verus/Kani execution in this state.
- No reviewer approval claimed.
