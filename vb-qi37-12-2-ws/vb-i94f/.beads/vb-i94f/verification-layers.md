# Verification Layers: Taint Propagation Through EvalExpr, BuildObject, BuildList, Choose, and Finish Paths

## Boundary

- **Verified kernel**: `vb_core` (pure hot execution engine — no async, no storage, no external I/O)
  - `crate::engine::expr_eval::core::eval_expr_inner` — pure deterministic bytecode evaluation with taint accumulation
  - `crate::engine::object_list::{build_object_with_taint, build_list_with_taint}` — deterministic collection construction with taint join
  - `crate::engine::choose::{choose_expr_branch, choose_slot_branch}` — deterministic branch selection
  - `crate::engine::node_helpers::{finish_run, copy_slot}` — deterministic finish and copy
  - `crate::engine::step::{eval_expr_node, build_object_node, build_list_node}` — IR node dispatch wrappers
  - `crate::frame::{read_taint, write_taint, write_slot_with_taint}` — slot taint read/write invariants
  - `crate::value::join_taint` — lattice join algebraic properties

- **Runtime shell**: `vb_runtime` orchestration layer (uses vb_core engine; exercises the above kernels deterministically via `step_once` in `drive_deterministic`)

- **External systems excluded from formal proof**: YAML authoring surface, Fjall persistence, IPC protocol, action dispatch (external ABI)

---

## Layer Assignment

### PRE-001 (RunFrame initialization before use)
- **Layer**: `miri` + `cargo-careful`
- **Rationale**: `miri` detects use of uninitialized memory and invalid enum/state discriminants. `cargo-careful` provides stricter UB checks on the frame construction path.
- **Checker**: `MIRIFLAGS="-Zstrict-provenance" cargo +nightly test -p vb_core -- --include-ignored` plus `cargo careful check -p vb_core`

### PRE-002 (CompiledWorkflow validated before use)
- **Layer**: `static-scan` + `compile-fail`
- **Rationale**: Workflow validation is a cold-path concern; `validate_compiled_workflow` is tested exhaustively in `vb_validate`. The runtime trust boundary is enforced by construction.
- **Checker**: `moon run :lint` (clippy deny + forbidden token scan)

### PRE-003 (ValueStore validity)
- **Layer**: `proptest` + `miri`
- **Rationale**: `ValueStore` operations are deterministic and pure; property tests cover arbitrary insertion ordering and handle reuse.
- **Checker**: `cargo nextest run -p vb_core --workspace` (proptest suite)

### PRE-004 (Slot index bounds)
- **Layer**: `kani` + `miri`
- **Rationale**: Every slot access is through `get`/`get_mut` with bounds checks returning typed errors. Kani can verify the absence of out-of-bounds access for all IR node dispatch paths.
- **Checker**: `kani --verify-rust-standard-library=false -p vb_core` on the frame/slot module; `cargo miri test -p vb_core`

### POST-001 (EvalExpr taint propagation)
- **Layer**: `lean` + `proptest` + `kani`
- **Rationale**:
  - `lean`: Prove that `eval_expr_inner`'s `taint_accum` at exit equals the lattice join of all `LoadSlot`/`LoadAccessor` taints consumed during execution. Theorem: `∀ plan run store expr. taint_accum = fold join_taint (slots read by expr)`.
  - `proptest`: Generate random expression programs with random slot taints and verify the joined result equals the accumulated taint.
  - `kani`: Bounded model check for expressions up to 10 ops, verifying `taint_accum` never decreases.
- **Lean target**: `vb_core/src/engine/expr_eval/core.rs` — prove `taint_accum_soundness`
- **Checker**: `lake build` (Lean) + `cargo nextest -p vb_core` (proptest) + `kani -p vb_core`

### POST-002 (BuildObject taint propagation)
- **Layer**: `lean` + `proptest`
- **Rationale**:
  - `lean`: Prove `accumulated_taint = fold join_taint (taint of each field slot)` for `build_object_with_taint`.
  - `proptest`: Generate random field lists with random taints, verify the returned taint is the join of all inputs.
- **Lean target**: `vb_core/src/engine/object_list.rs` — prove `build_object_with_taint_soundness`
- **Checker**: `lake build` + `cargo nextest -p vb_core`

### POST-003 (BuildList taint propagation)
- **Layer**: `lean` + `proptest`
- **Rationale**: Symmetric to POST-002 for list construction.
- **Lean target**: `vb_core/src/engine/object_list.rs` — prove `build_list_with_taint_soundness`
- **Checker**: `lake build` + `cargo nextest -p vb_core`

### POST-004 (Choose taint semantics)
- **Layer**: `proptest` + `static-analysis`
- **Rationale**: `Choose` does not accumulate taint (only one branch executes). Verify by exhaustive enumeration of branch condition outcomes for small branch counts.
- **Checker**: `cargo nextest -p vb_core` (choose branch tests)

### POST-005 (Finish taint propagation)
- **Layer**: `lean` + `proptest` + `kani`
- **Rationale**:
  - `lean`: Prove `EngineSignal::Finished(v, t)` emitted by `finish_run` carries `t = read_taint(result)` exactly.
  - `kani`: Bounded check that `finish_run` output taint matches input slot taint for all taint variants.
- **Lean target**: `vb_core/src/engine/node_helpers.rs` — prove `finish_run_preserves_taint`
- **Checker**: `lake build` + `kani -p vb_core`

### POST-006 (Copy slot taint preservation)
- **Layer**: `proptest` + `kani`
- **Rationale**: `copy_slot` reads both value and taint from source, writes both to destination. Property test and Kani verify equality.
- **Checker**: `cargo nextest -p vb_core` + `kani -p vb_core`

### POST-007 (Action completion taint)
- **Layer**: `proptest` + `manual-qa`
- **Rationale**: Action completion taint is a cross-boundary concern handled by `resume_action_completion`. The action ABI contract specifies taint propagation. Unit tests cover all taint variants.
- **Checker**: `cargo nextest -p vb_core` (resume_action_completion tests) + `moon run :ci` (integration)

### POST-008 (No taint desync)
- **Layer**: `miri` + `kani`
- **Rationale**: `write_slot_with_taint` writes both arrays atomically. `read_taint` on uninitialized slot returns `SlotUninitialized`. Miri verifies no use of uninitialized taint; Kani verifies the invariant for all sequences of write/read operations.
- **Checker**: `cargo miri test -p vb_core` + `kani -p vb_core`

### INV-001 (Taint monotonicity)
- **Layer**: `lean` + `kani`
- **Rationale**: Monotonicity of taint across run lifecycle. Lean: prove `join_taint` is monotone. Kani: bounded check that no code path decreases taint without `reinitialize`.
- **Checker**: `lake build` + `kani -p vb_core`

### INV-002 (Taint lattice soundness)
- **Layer**: `lean`
- **Rationale**: Algebraic proof that `join_taint` is commutative, associative, has `Secret` as top and `Clean` as bottom. Theorem in `vb_core/src/engine/expr_eval/core.rs` or a separate `lean/` proof module.
- **Checker**: `lake build`

### INV-003 (Slot/taint parallel arrays)
- **Layer**: `miri` + `kani`
- **Rationale**: `write_slot_with_taint` writes both `slots` and `taint` at the same index. Miri detects any split between them.
- **Checker**: `cargo miri test -p vb_core` + `kani -p vb_core`

### INV-004 (Object/List field taint immutability)
- **Layer**: `proptest`
- **Rationale**: `ValueStore::insert_object` / `insert_list` takes owned `ObjectField` / `SlotValue` slices. After insertion, the arena is immutable. Property tests verify that field taint is preserved in round-trip store/lookup.
- **Checker**: `cargo nextest -p vb_core` (value_store tests)

### INV-005 (Finish signal taint = result slot taint)
- **Layer**: `lean` + `kani`
- **Rationale**: Same as POST-005 — proven jointly.
- **Checker**: `lake build` + `kani -p vb_core`

### INV-006 (DerivedFromSecret is not Secret)
- **Layer**: `static-analysis` + `manual-qa`
- **Rationale**: Type-level distinction in the `Taint` enum. Runtime enforcement: `DerivedFromSecret` values are allowed in `Finish` output; only `Secret` triggers compile-time rejection. Manual QA verifies UI/log redaction at the `DerivedFromSecret` boundary.
- **Checker**: `moon run :lint` + manual verification of UI redaction behavior

### INV-007 (No untainted wrapper around tainted content)
- **Layer**: `proptest` + `ka
ni`
- **Rationale**: `build_object_with_taint` and `build_list_with_taint` accumulate taint across all inputs; the returned handle taint is the join. It is structurally impossible to produce a `Clean`-tainted container from `Secret`-tainted inputs. Kani verifies this impossibility for the construction functions.
- **Checker**: `kani -p vb_core` on `build_object_with_taint` and `build_list_with_taint`

---

## Lean Scope

- **Theorem module**: `vb_core/src/engine/expr_eval/` (pure deterministic kernel)
- **Rust target**: `eval_expr_inner`, `build_object_with_taint`, `build_list_with_taint`, `finish_run`, `join_taint`
- **Abstraction relation**:
  - Rust `SlotValue` ↔ Lean `Value` (ADT matching `Null | Bool | I64 | F64 | Symbol | List | Object | Blob`)
  - Rust `Taint` (`Clean | DerivedFromSecret | Secret`) ↔ Lean `Taint` (Lift of 3-element lattice)
  - Rust `SlotIdx` ↔ Lean `Fin n` (bounded index)
  - Rust `ExprProgram` ↔ Lean `List ExprOp` (bytecode as list)
  - `taint_accum = fold join_taint reads` ↔ Lean theorem statement
- **Non-goals**:
  - Proving Kani or Miri; those tools verify Rust directly
  - Proving the full `CompiledNodeKind` dispatch (only targeted nodes for this bead)
  - Proving `ValueStore` arena allocation correctness (out of scope for pure kernel)

---

## Waivers

- **Choose / ChooseSlot taint accumulation**: Explicit waiver. `Choose` does not produce or accumulate taint — it is a control-flow construct that selects one branch. The taint lattice join is only propagated through the branch that actually executes. Compensating evidence: `proptest` exhaustive branch coverage for small branch counts (≤ 5 branches) plus the existing unit tests in `choose.rs`.
- **Accessor taint propagation through path segments**: Explicit waiver for this bead. `eval_load_accessor` in `accessors.rs` already propagates taint via `taint_accum`. A future bead will formalize the accessor path. Compensating evidence: existing `proptest` in `expr_eval` module and unit tests in `accessors.rs`.
- **Replay/journal taint preservation**: Explicit waiver for this bead. Taint is persisted via `ActionJournalEvent::Completed { output_taint }` in the journal record envelope. A future bead will formalize replay soundness. Compensating evidence: integration tests covering journal replay in `vb_runtime`.
