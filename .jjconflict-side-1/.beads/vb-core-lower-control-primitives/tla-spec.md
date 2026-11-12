# TLA+ Temporal Model Plan

## Boundary
- **Temporal/workflow behavior**: Step chain well-formedness and structural reachability during lowering
- **Rust/core behavior excluded from TLA+**: Slot recording, node construction, `WaitKind` discriminant logic, arithmetic overflow checks
- **External systems abstracted**: None — this is a pure compilation transform
- **Non-applicability rationale**: The lowering is a pure function from validated YAML AST → vector of `CompiledNode`. There is no state-over-time behavior, no concurrency, no retry/lifecycle that persists beyond the lowering pass itself. The only "temporal-like" concern is the structural chain (Ask → AskResume, body/done offsets, join), which is a finite state machine that can be model-checked.

## TLA+-Owned Clauses
- TLA-WF-001 → specs/ControlLowering.tla::WellFormedStepChain
- TLA-WF-002 → specs/ControlLowering.tla::SlotVectorBounded

## Model Shape
- **Module/model path**: `specs/ControlLowering.tla`
- **Variables**:
  - `steps` — a sequence of node records: `[id, kind, output_slot, next_step]`
  - `slots` — a set of recorded `SlotIdx` values
- **Init action**: `Init ≡ steps = <<>> ∧ slots = {}`
- **Next/actions**:
  - `LowerForEach`, `LowerTogether`, `LowerCollect`, `LowerReduce`, `LowerRepeat`, `LowerAsk`, `LowerWait`
  - Each action appends the correct number of nodes and records required slots
- **State constraints**: `Len(steps) ≤ u16::MAX` (to reflect that `StepIdx` is `u16`)
- **Symmetry sets**: None — slot/step IDs are concrete values
- **Bounded model limits**: `MaxSteps = 10` for TLC; `MaxSlots = 20`

## Properties
- **Safety invariants**:
  - `NoDuplicateStepIds`: `∀ i, j ∈ DOMAIN steps: i ≠ j ⇒ steps[i].id ≠ steps[j].id`
  - `ValidOffsets`: `∀ n ∈ steps: n.kind ∈ {ForEachNext, RepeatAttempt} ⇒ n.body > n.id ∧ n.done > n.body`
  - `AskResumeIdCorrect`: `∀ i ∈ DOMAIN steps: steps[i].kind = AskResume ⇒ steps[i].id = steps[i-1].id + 1`
- **Liveness/eventuality**: Not applicable — lowering is a total pure function
- **Fairness assumptions**: Not applicable
- **Deadlock freedom**: Not applicable
- **Refinement to Rust/runtime behavior**: Each TLA+ action corresponds 1:1 to a Rust `lower_*` function; `steps` field names map directly to `CompiledNode` fields

## Evidence Command
```bash
tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla
```

## Waivers
- `repeat` max_attempts overflow — covered by Rust `checked_add` with explicit error, not modeled in TLA+
- `Together` branch count u16 limit — covered by Rust `u16::try_from` with explicit error, not modeled in TLA+
