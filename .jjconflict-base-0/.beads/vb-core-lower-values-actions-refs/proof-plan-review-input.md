# Proof Plan Review Input — vb-core-lower-values-actions-refs

## For: proof-reviewer skill

---

## Review Request

Review proof-obligations.planned.jsonl for vb-core-lower-values-actions-refs. The bead covers the `vb_compile` crate's lowering infrastructure for v1 values, slot/accessor references, action references, and taint metadata.

**Key architectural concern**: Slot/Accessor/Expression lowering is the foundation for the entire numeric IR pipeline. Bugs here are latent in all downstream validation gates.

---

## Critical Decisions to Validate

### 1. Verus waiver (WAIVER-VERUS-EXPR-STACK, WAIVER-VERUS-SLOT-MAX)

**Decision**: Mark VERUS-EXPR-STACK-001 and VERUS-SLOT-MAX-001 as `blocked_tooling`; use Kani + proptest as compensating coverage.

**Evidence for reviewer**:
- `crates/vb_compile/src/expression_bytecode.rs` has `#![forbid(unsafe_code)]` — expression bytecode is pure safe Rust
- `ExprProgram::try_from_ops` is a total pure function over bounded `ExprOp` sequences
- Stack effect is integer arithmetic over `i32` with `MAX_EXPRESSION_STACK = 16` (well within Kani's exhaust scope)
- `slot_count()` returns `u16` — all slot indices are u16-bounded

**Question for reviewer**: Is Kani 0.67.0 + proptest adequate compensation for Verus spec/proof fns on INV-004 (stack boundedness)? The contract says Verus is appropriate for "pure, total, integer inequality" — Kani can exhaust u16 bounds.

### 2. Optional obligations

**Decision**: `INV-007-NODEDUP-001` and `INV-006-ORDER-001` marked `required: false` because they are structural/data-structure properties of lowering determinism, not safety-critical. The proof-reviewer should confirm or reject this classification.

**Question for reviewer**: Are these truly optional, or does INV-007 (no duplicate StepIdx) constitute a regression risk if omitted?

### 3. vb-f04l blocker

**Decision**: Proof obligations target existing infrastructure (`expression_bytecode.rs`, `lib.rs SlotCompiler`), NOT the primitive-lowering callers. Reviewer should confirm no vb-f04l dependency exists in the Kani harness targets.

### 4. POST-009 (validation call)

**Decision**: `POST-009-VALIDATE-001` uses a unit test, not a Kani harness. The contract says `lower_steps_to_ir` "calls `vb_validate::shared::validate`" — this is a call-site property, not an arithmetic bound. A unit test that checks the resulting `CompiledWorkflow` is constructed is sufficient.

**Question for reviewer**: Is a unit test adequate for POST-009, or should a Kani harness verify the validation call is reached?

---

## Risk Tags in Scope

| Tag | Present? | Notes |
|-----|----------|-------|
| temporal | No | No loops/retry/state machines in expression bytecode |
| concurrency | No | No spawn/tokio/atomic in vb_compile lowering |
| unsafe/UB | No | `#![forbid(unsafe_code)]` in both files |
| parser/codec | Yes | Expression bytecode is a codec; Kani covers bounds |
| arithmetic/index | Yes | u16 slot indices, MAX_EXPRESSION_STACK, const pool overflow |
| dependency | No | vb-f04l is a blocks dependency, not a proof dependency |

---

## Files Under Review

| File | Line Count | Purpose |
|------|-----------|---------|
| `crates/vb_compile/src/expression_bytecode.rs` | ~400 | Expression→bytecode lowering |
| `crates/vb_compile/src/lib.rs` | ~4351 | SlotCompiler, lower_steps_to_ir, all lower_* functions |
| `crates/vb_core/src/expressions.rs` | ~200 | ExprProgram, ExprOp, stack_effect |
| `crates/vb_core/src/ids/kani_id_bounds.rs` | ~100 | Existing Kani bounds proofs |

---

## Reviewer Action

Run `proof-reviewer` on `proof-obligations.planned.jsonl` and confirm:
1. All 17 obligations are mapped to contract clauses
2. Waivers are acceptable for Verus obligations
3. Optional obligations are correctly classified
4. No unmapped proof obligations exist
