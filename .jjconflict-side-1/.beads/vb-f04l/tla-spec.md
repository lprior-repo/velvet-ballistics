# TLA+ Temporal Model Plan: vb-f04l

## Boundary

- Temporal/workflow behavior: lifecycle shape of lowered v1 primitives after source AST expansion: bounded `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, and `Ask` graph execution.
- Rust/core behavior excluded from TLA+: YAML parsing, exact Rust allocation, checked numeric conversion, hashing, serde/postcard, slot arithmetic, and direct proof of `compile_source` internals.
- External systems abstracted: action execution, wall-clock time, event delivery, human answer delivery, storage, and scheduler fairness.
- Model purpose: model-check the bounded lifecycle of prevalidated primitive graph shapes and require downstream bridge evidence tying emitted graph nodes, branch/body/done/join/resume targets, and slot metadata to this model. The current State5 model is lifecycle-only; graph-shape preservation remains separately required by Verus/Kani/proptest and implementation tests.

## TLA+-Owned Clauses

- POST-006 -> `verification/tla/V1PrimitiveLowering.tla::ForEachEventuallyDone` plus `TargetsInRange`.
- POST-007 -> `verification/tla/V1PrimitiveLowering.tla::TogetherEventuallyJoin` plus `NoPrematureTogetherJoin`.
- POST-008 -> `verification/tla/V1PrimitiveLowering.tla::CollectEventuallyDone` plus `PageNeverExceedsLimit`.
- POST-009 -> `verification/tla/V1PrimitiveLowering.tla::ReduceEventuallyDone` plus `TargetsInRange`.
- POST-010 -> `verification/tla/V1PrimitiveLowering.tla::RepeatEventuallyDone` plus `AttemptNeverExceedsMax`.
- POST-011 -> `verification/tla/V1PrimitiveLowering.tla::WaitEventuallyResumesOrTimesOut`.
- POST-012 -> `verification/tla/V1PrimitiveLowering.tla::AskEventuallyResumesOrTimesOut`.
- INV-002 -> `verification/tla/V1PrimitiveLowering.tla::TargetsInRange` and deadlock check.

## Model Shape

- Module/model path: `verification/tla/V1PrimitiveLowering.tla`.
- Config path: `verification/tla/V1PrimitiveLowering.cfg`.
- Variables: `primitive`, `phase`, `target`, `doneTarget`, `joinTarget`, `branches`, `completedBranches`, `attempt`, `maxAttempts`, `page`, `pageLimit`, `inputRemaining`, `suspended`, `delivered`, `answer`, `timedOut`, `finished`, `finishedCount`.
- Init action: `InitLoweredPrimitiveGraph` initializes one bounded primitive lifecycle with finite model nodes and continuation targets.
- Next/actions: `Start`, `EnterBody`, `BodyDone`, `AdvanceLoop`, `CompleteLoop`, `StartBranches`, `CompleteBranch`, `JoinBranches`, `PageDone`, `CompleteCollect`, `AttemptDone`, `RetryOrFinish`, `Suspend`, `DeliverEvent`, `DeliverAnswer`, `Timeout`, `FinishPrimitive`.
- State constraints: `MaxNodes = 12`, `BranchBound = 4`, `AttemptBound = 4`, `PageBound = 4`, `InputBound = 4`; targets range over `0..MaxNodes-1`.
- Symmetry sets: branch IDs are semantically symmetric for `Together`; TLC config does not currently declare symmetry reduction.
- Bounded model limits: one primitive lifecycle at a time. Continuation composition and emitted-node-array validation are downstream proof/test obligations, not claimed by this TLA+ model.

## Properties

- Safety invariants:
  - `TargetsInRange`: `target`, `doneTarget`, and `joinTarget` are model nodes.
  - `NoPrematureTogetherJoin`: `Together` cannot reach done until `completedBranches = branches`.
  - `AttemptNeverExceedsMax`: repeat attempts never exceed `maxAttempts`.
  - `PageNeverExceedsLimit`: collect pages never exceed `pageLimit`.
  - `SingleCompletion`: each primitive reaches completion at most once.
  - `BranchCountBounded`: emitted branch count stays inside the configured finite bound.
- Liveness/eventuality:
  - `ForEachEventuallyDone` under finite input/body completion.
  - `TogetherEventuallyJoin` under weak fairness for branch completion and join.
  - `CollectEventuallyDone` under finite page source.
  - `ReduceEventuallyDone` under finite input/body completion.
  - `RepeatEventuallyDone` under bounded attempts.
  - `WaitEventuallyResumesOrTimesOut` under event-delivery or timeout fairness.
  - `AskEventuallyResumesOrTimesOut` under answer-delivery or timeout fairness.
- Fairness assumptions: `WF_vars` on `Start`, body completion, loop completion, branch completion, join, page completion, collect completion, attempt completion, retry finish, suspend, event delivery, answer delivery, and timeout actions as encoded in `Spec`.
- Deadlock freedom: `CHECK_DEADLOCK TRUE`; no nonterminal bounded state may deadlock.
- Refinement to Rust/runtime behavior: downstream proof-writer must provide a bridge relation from each emitted `CompiledNodeKind` family and `StepIdx` target set to this model's `primitive`, target, counter, and suspend/resume variables. TLA+ evidence alone is not allowed to claim concrete emitted graph shape preservation.

## Evidence Command

- `tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`
- Expected evidence: TLC exits 0 and reports `Model checking completed. No error has been found.` with all configured invariants, liveness properties, and `CHECK_DEADLOCK TRUE` satisfied.

## Waivers

- None for `Collect`, `Reduce`, `Wait`, or `Ask`; each has a required TLA+ obligation in `proof-obligations.jsonl`.
