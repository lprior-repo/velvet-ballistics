# Verification Layers: vb-a001

## Boundary

- **Verus-owned kernel:** `lower_canonical_for_each` edge emission (INV-002), `drive_deterministic_full` termination (INV-004).
- **TLA+ temporal model:** compiled-parity (INV-005), for_each termination (INV-004 temporal).
- **Theorem projection:** none — Verus owns all Rust-local proof obligations.
- **Runtime shell:** CLI dispatch, journal I/O, slot store, postcard serialization/deserialization.
- **External systems excluded:** filesystem (postcard artifacts), database (journal), wall-clock time.

## Layer Assignment

| Clause | Layers |
|--------|--------|
| PRE-001..PRE-005 (lowering + validation) | `static-scan` + `fowler-tests` + `kani` |
| PRE-007..PRE-008 (runtime primitives) | `fowler-tests` + `proptest` |
| POST-001 (compiled parity happy path) | `fowler-tests` + `manual-qa` + `tla-plus` |
| POST-002 (empty list) | `fowler-tests` |
| POST-003 (validation rejection) | `fowler-tests` + `kani` |
| POST-004 (artifact round-trip) | `fowler-tests` + `cargo-fuzz` |
| INV-001 (lowering node count) | `verus` + `fowler-tests` |
| INV-002 (edge invariant) | `verus` + `kani` + `proptest` |
| INV-003 (reachability) | `verus` + `fowler-tests` |
| INV-004 (termination) | `verus` + `tla-plus` + `fowler-tests` |
| INV-005 (parity) | `tla-plus` + `fowler-tests` + `manual-qa` |

## Verus Scope

- **Rust targets:**
  - `vb_compile::mod_compile_lowering::part_02::lower_canonical_for_each` — spec function asserting emitted node count, node order, and body SetConst next-edge value.
  - `vb_runtime::engine::drive_deterministic_full` — loop invariant with decreasing measure (iterator list length + remaining iterations).
  - `vb_core::workflow::validate_reachability` — spec function asserting that `Ok(())` implies all nodes reachable.
  - `vb_core::workflow::validate_forward_edges` — spec function asserting that forward-only edges hold when `Ok(())`.
- **Spec/proof functions:**
  - `spec_lower_canonical_for_each` — abstract model: input → [ForEachStart, SetConst, ForEachNext, SetConst].
  - `proof_body_setconst_next_is_foreachnext` — proves body SetConst.next = Some(ForEachNext_step) where ForEachNext_step > body_step.
  - `spec_drive_loop_invariant` — loop invariant: PC ∈ nodes, remaining_iter ≥ 0.
- **Trusted boundary:** `SlotCompiler` state (nodes vec, constants, slots) — the spec model abstracts the concrete SlotCompiler into a list of (StepIdx, CompiledNodeKind, Option<StepIdx>) tuples.
- **Shell exclusions:** I/O, postcard serialization, journal writing, CLI argument parsing.

## TLA+ Scope

- **Module/model:** `specs/ForEachParity.tla` (new file, path TBD).
- **Variables:**
  - `nodes: SET OF NodeID` — set of node indices.
  - `edgeMap: NodeID ↦ NodeID ∪ {∅}` — next-edge mapping (abstracted; body/done edges handled separately).
  - `pc: NodeID` — current PC.
  - `iterList: Seq(Value)` — current iterator list.
  - `journal: Seq(JournalEvent)` — journal event sequence.
  - `runDone: BOOL` — whether run has completed.
- **Actions:**
  - `Init` — pc = entry, iterList = input list, journal = ⟨⟩, runDone = FALSE.
  - `ExecuteNode` — advances PC according to node kind. For ForEachStart/Next: pops from iterList or advances to done. For SetConst/Finish: advances to next.
  - `EmitRunFinished` — appends RunFinished to journal, sets runDone = TRUE.
- **Safety invariants:**
  - `PCInBounds`: pc ∈ nodes ∪ {∅}.
  - `JournalMonotonic`: journal grows only by appending.
  - `Parity`: runPath.journal = compiledPath.journal.
- **Temporal properties:**
  - `EventuallyFinished`: □(¬runDone ⇒ ◇runDone) — every run terminates.
  - `ParityPreserved`: □(runDone ⇒ runPath.journal = compiledPath.journal).
- **Fairness:** weak fairness on ExecuteNode (the loop must progress).
- **Deadlock stance:** deadlock freedom guaranteed by INV-004 termination proof.
- **Refinement boundary:** Rust `drive_deterministic_full` refines TLA+ `ExecuteNode` by mapping CompiledNodeKind variants to TLA+ action transitions. The concrete slot store and value types are abstracted to TLA+ sets/sequences.
- **Evidence command:** `tlc -config specs/ForEachParity.cfg specs/ForEachParity.tla` (exact path TBD).

## Theorem Scope

- Not applicable. Verus owns all Rust-local proof obligations. No theorem kernel projection needed.

## Waivers

- None. All contract clauses map to at least one verification layer.
