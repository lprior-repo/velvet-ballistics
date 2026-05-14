# Implementation Summary: vb-i94f

## Status: COMPLETE

All tests pass (`cargo test -p vb_core`: **1520 passed**).

---

## What Exists

### Core Taint Infrastructure

| Module | Location | Key Items |
|--------|----------|-----------|
| `Taint` enum + `join_taint` | `crates/vb_core/src/value.rs` | Three-level lattice: `Clean < DerivedFromSecret < Secret` |
| `RunFrame` slot/taint ops | `crates/vb_core/src/frame.rs` | `read_taint`, `write_taint`, `write_slot_with_taint` — atomic slot+taint writes |

### Expression Evaluation (POST-001)
- **Implementation**: `crates/vb_core/src/engine/expr_eval/core.rs` — `eval_expr_inner`
- **Taint accumulation**: `taint_accum` starts `Clean` and joins via `join_taint` on every `LoadSlot`/`LoadAccessor` op
- **Tests**: 54 tests in `integration_taint_propagation.rs` covering all B-030–B-038 behaviors

### BuildObject Taint (POST-002)
- **Implementation**: `crates/vb_core/src/engine/object_list.rs` — `build_object_with_taint`
- **Accumulation**: Iterates all field slots, joins their taints via `join_taint`, stores `ObjectField { value, taint }` per field
- **Tests**: B-040–B-047

### BuildList Taint (POST-003)
- **Implementation**: `crates/vb_core/src/engine/object_list.rs` — `build_list_with_taint`
- **Accumulation**: Iterates all item slots, joins their taints via `join_taint`
- **Tests**: B-050–B-057

### Choose Taint Semantics (POST-004)
- **Implementation**: `crates/vb_core/src/engine/choose.rs` — `choose_expr_branch`, `choose_slot_branch`
- **Design**: No taint accumulated — branch condition is boolean, only selected branch executes
- **Tests**: B-060–B-068

### Finish Taint (POST-005)
- **Implementation**: `crates/vb_core/src/engine/node_helpers.rs` — `finish_run`
- **Propagation**: Reads `read_slot` + `read_taint` from result slot, emits `EngineSignal::Finished(value, taint)`
- **Tests**: B-070–B-073, B-140–B-141

### Copy Slot Taint (POST-006)
- **Implementation**: `crates/vb_core/src/engine/node_helpers.rs` — `copy_slot`
- **Preservation**: Reads both value and taint from source, writes both via `write_slot_with_taint`
- **Tests**: B-080–B-084

### No Taint Desync (POST-008, INV-003)
- **Mechanism**: `write_slot_with_taint` atomically writes both `slots[i]` and `taint[i]`
- **Guard**: `write_taint` rejects uninitialized slots (prevents taint without value)
- **Guard**: `read_taint` returns `SlotUninitialized` for uninitialized slots (prevents stale taint)

---

## Test Coverage

| Behavior Group | Count | Location |
|---------------|-------|----------|
| join_taint lattice algebra (B-001–B-007) | 7 | `integration_taint_propagation.rs:50` |
| Frame slot operations (B-010–B-020) | 11 | `integration_taint_propagation.rs:149` |
| EvalExpr taint (B-030–B-038) | 9 | `integration_taint_propagation.rs` |
| BuildObject taint (B-040–B-047) | 8 | `integration_taint_propagation.rs` |
| BuildList taint (B-050–B-057) | 8 | `integration_taint_propagation.rs` |
| Choose taint semantics (B-060–B-068) | 9 | `integration_taint_propagation.rs` |
| Finish taint (B-070–B-073) | 4 | `integration_taint_propagation.rs` |
| Copy slot taint (B-080–B-084) | 5 | `integration_taint_propagation.rs` |
| Error handling (B-200–B-211) | 12 | `integration_taint_propagation.rs` |
| Misc invariants (B-100–B-151) | ~20 | `integration_taint_propagation.rs` |

**Total**: 54 dedicated taint propagation tests.

---

## What Still Needs Doing (Future Beads)

1. **Lean proofs** — Formal proofs for `taint_accum_soundness`, `build_object_with_taint_soundness`, `build_list_with_taint_soundness`, `finish_run_preserves_taint` (see `verification-layers.md` POST-001, POST-002, POST-003, POST-005)
2. **Kani harnesses** — Bounded model check harnesses for `eval_expr_taint_monotone`, `finish_run_taint`, `copy_slot_taint`, `container_taint_impossible`, etc. (see `proof-obligations.jsonl`)
3. **Proptest expansions** — Property-based tests for random `ExprProgram` generation with random slot taints
4. **Accessor taint path** — `eval_load_accessor` in `accessors.rs` accumulates taint but formal proof deferred to future bead
5. **Replay/journal taint preservation** — Deferred to future bead per waiver in `verification-layers.md`

---

## Gates Run

```bash
cargo test -p vb_core  # 1520 passed (6 suites)
```

No formal verification (Lean/Kani) has been run yet — those are deferred to separate proof-lane gates per `verification-layers.md`.

---

## Residual Risk

- **Formal proofs incomplete**: Lean proofs for lattice soundness (INV-002) and taint monotonicity (INV-001) are deferred
- **Kani not executed**: Bounded model checking for container taint impossibility (INV-007) and error handling exhaustiveness not yet run
- **Accessor path**: Path-segment taint propagation in `eval_load_accessor` has unit test coverage but no formal proof

These are out of scope for this bead per the contract's non-goals and explicit waivers.
