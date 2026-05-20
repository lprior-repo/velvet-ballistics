# TLA+ Temporal Model Plan

## Boundary

- **Temporal/workflow behavior:**
  - INV-005 (compiled-parity): both `run` and `run-compiled` paths produce identical journal event sequences.
  - INV-004 (termination): for_each loop terminates for any finite input list within the `limit` bound.
- **Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests:**
  - Slot store implementation, value types, expression evaluation, constant pool management.
  - Postcard serialization/deserialization.
  - Forward-edge validation logic (INV-002 edge ordering — Verus-owned).
- **External systems abstracted:**
  - Filesystem (postcard artifact I/O) — modeled as an atomic read/write of the IR graph.
  - Journal database — modeled as an append-only sequence `journal`.
  - Wall-clock time — not modeled; termination is logical, not real-time.

## TLA+-Owned Clauses

- **INV-005 → specs/ForEachParity.tla::ParityPreserved**
  - Both execution paths (interpreted and compiled) produce identical journal event sequences.
- **INV-004 → specs/ForEachParity.tla::EventuallyFinished**
  - For any finite input list and finite limit, the loop eventually reaches RunFinished.

## Model Shape

- **Module/model path:** `specs/ForEachParity.tla` (path TBD — may live in `formal-verification/` or repo root `specs/`).
- **Variables:**
  - `nodes`: finite set of node indices `{0, 1, ..., N}`.
  - `nextEdge`: partial function `nodes → nodes ∪ {∅}` — abstracts the `next` field.
  - `bodyEdge`: partial function `nodes → nodes ∪ {∅}` — abstracts loop body targets.
  - `pc`: `nodes ∪ {∅}` — current program counter.
  - `iterList`: Seq(Value) — remaining iterator list at ForEachNext.
  - `journal`: Seq(JournalEvent) — journal event log.
  - `runDone`: BOOL — termination flag.
  - `limit`: Nat — for_each limit bound.
  - `phase`: {"interpreted", "compiled"} — distinguishes the two execution paths.
- **Init action:** `Init` — pc = entry (node 0), iterList = InputList, journal = ⟨⟩, runDone = FALSE, limit = InputLimit.
- **Next/actions:**
  - `ExecNop` — pc := nextEdge[pc].
  - `ExecSetConst` — pc := nextEdge[pc].
  - `ExecForEachStart` — if Len(iterList) = 0 then pc := bodyEdge[pc] (done path); else iterList := Tail(iterList), pc := bodyEdge[pc] (body path).
  - `ExecForEachNext` — if Len(iterList) = 0 then pc := bodyEdge[pc] (done path); else iterList := Tail(iterList), pc := bodyEdge[pc].
  - `ExecFinish` — journal := Append(journal, ⟨"RunFinished", runID⟩), runDone := TRUE.
  - `EmitRunFinished` — journal := Append(journal, ⟨"RunFinished", runID⟩), runDone := TRUE.
- **State constraints:** `nodes` is finite (bounded by max_step u16 limit = 65535 for TLC). `Len(iterList) ≤ limit`.
- **Symmetry sets:** none (node indices are ordered).
- **Bounded model limits:** nodes ≤ 20, limit ≤ 5, iterList ≤ 5 elements (TLC tractability).

## Properties

- **Safety invariants:**
  - `PCInBounds`: pc ∈ nodes ∪ {∅}.
  - `ValidIterList`: iterList is a prefix of InputList (items not duplicated or lost).
  - `JournalConsistent`: every journal entry has a unique sequence number.
- **Liveness/eventuality:**
  - `EventuallyFinished`: □(¬runDone ⇒ ◇runDone) — the run eventually terminates.
  - `NoInfiniteLoop`: ◇(runDone ∧ Len(journal) ≤ limit + 10) — loop count bounded.
- **Fairness assumptions:** weak fairness on `ExecNop`, `ExecForEachStart`, `ExecForEachNext`, `ExecFinish` under enabled actions.
- **Deadlock freedom:** proved by the `EventuallyFinished` liveness property with weak fairness on all executable actions. The only "stuck" state is `runDone = TRUE`.
- **Refinement to Rust/runtime behavior:**
  - TLA+ `ExecForEachStart` refines Rust `for_each_start`: both check input list, branch on empty/non-empty, advance to body or done.
  - TLA+ `ExecForEachNext` refines Rust `for_each_next`: both check iterator list, branch on empty/non-empty, advance to body or done.
  - TLA+ `pc` maps to Rust `RunFrame.pc`.
  - TLA+ `journal` maps to Rust `RuntimeJournalEvent` append sequence.
  - The abstraction relation: `AbstractSlotStore = concrete ValueStore projected to Seq(Value)` for iterator list modeling.

## Evidence Command

- **TLC:** `tlc -config specs/ForEachParity.cfg specs/ForEachParity.tla` (exact path TBD).
- **Apalache (alternative):** `apalache-mc check --config specs/ForEachParity.cfg specs/ForEachParity.tla` — for unbounded list length checking if TLC bounds prove insufficient.

## Waivers

- None. Both INV-004 and INV-005 are temporal/state-over-time properties that require TLA+ for model checking.
