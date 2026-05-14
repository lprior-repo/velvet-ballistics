# TLA+ Temporal Model Plan: Generated Rust Semantic Parity

> Status note: this file captures the fuller desired temporal model. The executable
> model currently committed at `specs/GeneratedParity.tla` is a smaller valid-lifecycle
> abstraction. Claims for invalid-resume no-mutation and concrete journal no-drop are
> intentionally owned by Verus/Kani/Rust tests in this revision, not by TLC output.

## Boundary
- Temporal/workflow behavior: generated run lifecycle for deterministic steps, `Do` suspension/resume, `Ask` suspension/resume, journal event ordering, budget exhaustion, terminal `RunFinished`, and typed failure.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests: concrete array indexing, Rust enum layout, exact generated source text, postcard/envelope encoding, taint join implementation details.
- External systems abstracted: action executor returns `Ready`, `Suspended`, `Completed`, or `Failed`; ask system returns an answer payload; journal is an append-only bounded sequence.
- Non-applicability rationale: not applicable; this bead is temporal/state-over-time by nature.

## TLA+-Owned Clauses
- POST-003 -> `GeneratedParity::ActionScheduledAtDoSuspend`
- POST-004 -> `GeneratedParity::ActionResumeWritesAndCompletes`
- POST-005 -> `GeneratedParity::AskResumeWritesAnswer`
- POST-006 -> `GeneratedParity::JournalOrderEquivalent`
- INV-003 -> desired `GeneratedParity::NoJournalOverflowDrop`; current TLC evidence covers only abstract capacity-error transition, while concrete no-drop proof is Verus/Rust-owned.
- INV-005 -> desired `GeneratedParity::ResumeIdentityCheckedBeforeMutation`; current TLC evidence does not claim invalid-resume preservation, which is Verus/Kani/Rust-owned.
- INV-006 -> `GeneratedParity::ObservationalTraceParity`

## Model Shape
- Desired module/model path: `.beads/vb-gvmt/specs/GeneratedParity.tla`
- Desired config path: `.beads/vb-gvmt/specs/GeneratedParity.cfg`
- Current status: executable model/config exist, but they are the smaller valid-lifecycle abstraction described above.
- Variables:
  - `pc`: current step or terminal marker.
  - `slots`: finite map `Slot -> Value`.
  - `taints`: finite map `Slot -> Taint`.
  - `journal`: sequence of event records.
  - `pendingAction`: `None` or record `{run, step, action, inputSlot, outputSlot, ticket, resumePc}`.
  - `pendingAsk`: `None` or record `{run, step, promptSlot, answerSlot, ticket, resumePc}`.
  - `budget`: remaining step budget.
  - `terminal`: `None`, `Finished(value, taint)`, or `Error(kind, fields)`.
  - `irTrace`: abstract IR observation sequence for the same fixture.
  - `genTrace`: generated observation sequence.
- Init action: `Init` creates finite slots/taints, empty journal, no pending action/ask, non-terminal pc at first step, bounded budget.
- Next/actions:
  - `DeterministicSlotWrite`
  - `DoSuspend`
  - `DoResumeValid`
  - `DoResumeInvalid`
  - `AskSuspend`
  - `AskResumeValid`
  - `AskResumeInvalid`
  - `FinishRun`
  - `BudgetExhaust`
  - `JournalCapacityFail`
- State constraints:
  - `Slot = 0..MaxSlots-1`, `Step = 0..MaxSteps-1`, bounded tickets, bounded journal capacity, finite value set `{Null, I64_0, I64_41, I64_42, Symbol_9001}`.
  - Taints `{Clean, DerivedFromSecret, Secret}`.
- Symmetry sets: none required initially; finite constants are already small.
- Bounded model limits: start with `MaxSteps <= 5`, `MaxSlots <= 4`, `MaxJournal <= 8`, `MaxTickets <= 2`, `MaxBudget <= 6`.

## Properties
- Safety invariants:
  - `SlotTaintParallel`: every written slot has a corresponding taint value.
  - `JournalAppendOnly`: journal is append-only and never reorders prior events.
  - `ActionScheduleBeforeComplete`: `ActionCompleted` for a ticket appears only after matching `ActionScheduled`.
  - `AskAnswerBeforeAdvance`: `AskResumeValid` writes answer slot and optional `AskAnswered` before advancing beyond resume pc.
  - `RunFinishedLast`: once `RunFinished` appears, no later event for that run is appended.
  - `NoMutationOnInvalidResume`: invalid resume preserves slots, taints, pc, and pending state.
  - `NoDropOnJournalFull`: when required event would exceed capacity, terminal is a journal capacity typed error and the event is not silently dropped.
  - `TraceParity`: normalized generated observations equal normalized IR observations for modeled fixtures.
- Temporal properties:
  - `EventuallyTerminalOrSuspended`: under weak fairness, a non-blocked run eventually reaches `Finished`, a typed error, or an explicit suspension.
  - `ScheduledEventuallyCompletable`: if matching valid completion remains enabled for a pending action, eventually `ActionCompleted` or typed rejection occurs.
  - `AskEventuallyAnswerable`: if matching valid ask answer remains enabled, eventually answer slot is written and run advances or typed rejection occurs.
- Fairness assumptions:
  - Weak fairness on valid resume actions when their matching payload remains continuously enabled.
  - No fairness assumed for external actor producing payload; model abstracts both absent and present payload cases.
- Deadlock freedom: TLC must report no deadlock except states intentionally modeled as external suspension (`pendingAction` or `pendingAsk` with no payload) or terminal.
- Refinement to Rust/runtime behavior:
  - Runtime/generated journal event kinds refine TLA+ event tags by `{SlotWritten, ActionScheduled, ActionCompleted, AskAnswered?, RunFinished}`.
  - Generated `StepOutcome`/`DriveError` refines `terminal`.
  - Generated slots/taints refine `slots`/`taints` after each transition.

## Evidence Command
- BLOCKED: no exact `tlc` command is valid until `.beads/vb-gvmt/specs/GeneratedParity.tla` and `.cfg` exist.
- Desired command after files exist: `tlc -config .beads/vb-gvmt/specs/GeneratedParity.cfg .beads/vb-gvmt/specs/GeneratedParity.tla`
- Expected evidence after implementation: TLC exits 0, reports no invariant violations, no unexpected deadlock, and temporal properties satisfied within configured bounds.

## Waivers
- `AskAnswered` event naming is a waiver candidate only if the runtime journal schema definitively lacks a distinct `AskAnswered` event. Compensating evidence then must prove ask answer is observable through `SlotWritten` plus ask-specific resume metadata and parity tests.
