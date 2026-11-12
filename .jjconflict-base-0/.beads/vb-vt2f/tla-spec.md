# vb-vt2f TLA+ Temporal Model Plan

## Boundary

- Temporal behavior in product domain: submit, inspect, cancel, action completion/failure, ask answer, trace drain/list, graceful shutdown, strict admission, and accepted-artifact gating are lifecycle/protocol transitions over runs and admission state.
- Current stance: TLA+ is required for vb-vt2f temporal proof coverage. Earlier BDD-only TLA waivers are superseded audit records and are not approval paths.
- Rust/core behavior excluded from TLA+: concrete Rust facade/shard implementation details, Kani projection-kernel internals, BDD fixture construction, store engine internals, and runtime shell effects.
- External systems abstracted: wall clock, filesystem, Fjall/storage internals, OS scheduling, and non-deterministic IO. Bounded model counters and error states are modeled rather than assumed away.

## TLA+-Owned Clauses

- `TLA-VT2F-LIFECYCLE-001` covers `POST-004` through `POST-011` and `INV-001`.
- `TLA-VT2F-STRICT-ADMISSION-001` covers `POST-012`, `ERR-002`, `PRE-005`, and `INV-006`.
- `WAIVER-TLA-VT2F-001` and `WAIVER-TLA-VT2F-002` are superseded and must not be counted as approval evidence.

## Model Shape

### Vt2fRuntimeLifecycle

- Module/model path: `verification/tla/Vt2fRuntimeLifecycle.tla` with config `verification/tla/Vt2fRuntimeLifecycle.cfg`.
- Variables: `runs`, `queue`, `journal`, `trace`, `counters`, `steps`, `shutdown`, `errors`.
- Init action: `Init`.
- Next/actions: `Submit`, `Tick`, `Inspect`, `Cancel`, `CompleteAction`, `FailAction`, `AnswerAsk`, `ListTrace`, `DrainTrace`, `Shutdown`, `ErrorTransition`.
- State constraints: finite runs, queue, journal, trace, counter, step, shutdown, and typed-error domains as configured by the `.cfg` file.
- Bounded hardware stance: bounded counters and overflow/error transitions are part of the model contract; unbounded `Nat` success must not be used to hide arithmetic failure.

### Vt2fStrictAdmission

- Module/model path: `verification/tla/Vt2fStrictAdmission.tla` with config `verification/tla/Vt2fStrictAdmission.cfg`.
- Variables: `policy`, `store_mode`, `accepted_digests`, `capabilities`, `submit_queue`, `admission_result`, `runtime_constructor`.
- Init action: `Init`.
- Next/actions: `ConfigureRelaxed`, `ConfigureStrict`, `ConfigureJournaled`, `AcceptArtifact`, `SubmitDirect`, `RejectMissingArtifact`, `EnqueueAccepted`, `ConstructShardWithExplicitStore`, `ConstructRuntimeWithoutStore`.
- State constraints: finite digest/capability/store/policy domains and bounded submit/admission transitions.

## Properties

### Vt2fRuntimeLifecycle

- Safety invariants: `NoWrongRunMutation`, `TraceListNonDestructive`, `DrainTraceDestructive`, `CancellationRemovesActiveRun`, `ShutdownNoFurtherProgress`, `DeterministicTickOutcome`, `BoundedCountersNoOverflow`.
- Liveness/eventuality: `EventuallyTerminalOrSuspendedOrTypedErrorWithinBounds`.
- Fairness assumptions: weak fairness on enabled tick/control actions within finite TLC bounds; heartbeat-only stutter is not an approval path.
- Deadlock freedom: `NoDeadlockWithoutHeartbeatMask`.
- Refinement to Rust/runtime behavior: public `Runtime` facade observations refine abstract TLA+ actions by `RunId`, ticket, state class, trace event class, shutdown state, and typed error class. Concrete implementation proof is handled by BDD, Kani projection kernels, and review obligations.

### Vt2fStrictAdmission

- Safety invariants: `StrictMissingStoreRejectsBeforeEnqueue`, `JournaledMissingStoreRejectsBeforeEnqueue`, `AcceptedDigestCapabilityRequired`, `RelaxedModeSeparate`, `ExplicitShardStoreNotRuntimeMissingStore`, `NoMissingArtifactBypass`.
- Liveness/eventuality: `EverySubmitEventuallyAcceptedOrTypedRejectedWithinBounds`.
- Fairness assumptions: weak fairness on enabled submit/reject/enqueue/admission progress actions within finite TLC bounds.
- Deadlock freedom: no complete-state deadlock for configured finite bounds.
- Refinement to Rust/runtime behavior: strict/journaled admission and accepted-store modes refine public admission behavior and BDD/Kani projection-kernel observations; store implementation details remain outside TLA+.

## Evidence Commands And Current Evidence

- `tlc -config verification/tla/Vt2fRuntimeLifecycle.cfg verification/tla/Vt2fRuntimeLifecycle.tla` -> PASS per `.beads/vb-vt2f/proof-evidence.md`, with no errors over 1302 distinct states and the lifecycle properties above.
- `tlc -metadir states/vt2f-strict-admission-attempt7 -config verification/tla/Vt2fStrictAdmission.cfg verification/tla/Vt2fStrictAdmission.tla` -> PASS per `.beads/vb-vt2f/proof-evidence.md`, with no errors over 1096 distinct states and `EverySubmitEventuallyAcceptedOrTypedRejectedWithinBounds` checked.
- Contract obligation command remains `tlc -config verification/tla/Vt2fStrictAdmission.cfg verification/tla/Vt2fStrictAdmission.tla`; the `-metadir` evidence command is an execution detail from the passing run.

## Waivers

- No active TLA+ waiver for vb-vt2f temporal clauses.
- Historical `WAIVER-TLA-VT2F-001` and `WAIVER-TLA-VT2F-002` are superseded, retained only for audit, and not approval paths.
