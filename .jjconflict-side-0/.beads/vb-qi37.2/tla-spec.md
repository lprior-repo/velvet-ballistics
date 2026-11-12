# TLA+ Temporal Model Plan: vb-qi37.2

## Boundary
- Temporal/workflow behavior: workflow validation produces a bound certificate; admission reserves aggregate capacity before acknowledgment; runtime creates capped run state; deterministic execution consumes step budget; over-budget paths fail closed.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/proptest/fuzz: checked arithmetic for budget totals, monotonicity, ValueStore cap arithmetic, StepBudget counter arithmetic.
- External systems abstracted: storage, CLI, YAML compiler, wall-clock, scheduler, and filesystem.

## TLA+-Owned Clauses
- POST-001: accepted workflow admission has a deterministic bound certificate.
- POST-002 / INV-002: requested aggregate usage fits capacity before runnable acknowledgment.
- POST-005 / INV-007: step exhaustion reaches deterministic typed outcome.
- POST-006: fail-closed outcomes do not transition to runnable/acknowledged states.

## Model Shape
- Module/model path: `verification/tla/WorkflowBoundedAdmission.tla` and `verification/tla/WorkflowBoundedAdmission.cfg` (planned; not authored in State 3).
- Variables: `artifactState`, `certificate`, `requestedBudget`, `capacity`, `usage`, `reservation`, `runState`, `valueSlots`, `stepBudget`, `outcome`.
- Init action: `Init` initializes unvalidated artifact, empty certificate/reservation, finite capacity, zero usage, no run state, finite step budget.
- Next/actions: `ComputeCertificate`, `RejectInvalidCertificate`, `ReserveCapacity`, `RejectOverCapacity`, `AckRun`, `CreateCappedRunState`, `ExecuteStep`, `ExhaustStepBudget`, `ReleaseReservation`, `FailClosed`.
- State constraints: finite artifacts, finite budget dimensions, finite capacity, bounded `MaxSlots`, bounded `MaxSteps`, bounded state set for TLC.
- Symmetry sets: artifacts/runs may be symmetric by run id for bounded TLC configs.
- Bounded model limits: at least 2 runs, 2 artifacts, capacity less than sum of some requested budgets, max step budget 0..3, max slots 0..3 to force rejection/exhaustion paths.

## Properties
- Safety invariants:
  - `NoAckWithoutCertificate`: `Acked => certificate.valid`.
  - `NoAckOverCapacity`: `Acked => usage + requestedBudget <= capacity`.
  - `NoUncappedRunState`: `RunStateCreated => valueSlots.cap <= certificate.maxSlots`.
  - `FailClosedNotRunnable`: rejected/failed artifacts never become runnable without a new valid certificate and reservation action.
  - `StepBudgetNeverNegative`: step budget counter never goes below zero.
- Liveness/eventuality:
  - `EventuallyAckOrReject`: every submitted artifact eventually reaches acked or rejected under fair certificate/capacity actions.
  - `EventuallyBlockedOrTerminal`: every acked run with finite step budget eventually blocks on budget exhaustion or reaches a terminal/runtime-blocked outcome under fair execute actions.
- Fairness assumptions: weak fairness on `ComputeCertificate`, `ReserveCapacity`, `ExecuteStep`, and `ReleaseReservation` when enabled; no fairness assumed for external storage/CLI.
- Deadlock freedom: no deadlock except explicit terminal/rejected states.
- Refinement to Rust/runtime behavior: Rust validation/refinement events map to `ComputeCertificate`; `admit_run_with_budget` maps to `ReserveCapacity`/`RejectOverCapacity`; `RunAdmission::with_budget` maps to `AckRun`; `ValueStore::with_max_slots` maps to `CreateCappedRunState`; `run_until_blocked`/`drive_deterministic` maps to `ExecuteStep` and `ExhaustStepBudget`.

## Evidence Command
- Planned exact command after model files exist: `tlc -config verification/tla/WorkflowBoundedAdmission.cfg verification/tla/WorkflowBoundedAdmission.tla`
- Expected evidence: TLC reports no invariant violations, no deadlock outside explicit terminal states, and temporal properties satisfied for bounded model limits.

## Waivers
- None. Temporal admission/reservation/execution lifecycle applies and requires TLA+ or an approved reviewer waiver.
