# Black Hat Review — vb-core-lower-values-actions-refs

**Bead**: vb-core-lower-values-actions-refs
**Workspace**: /tmp/vb-ws/vb-core-lower-values-actions-refs
**Reviewer**: black-hat-reviewer (state 12)
**Date**: 2026-05-15

---

## STATUS: APPROVED

---

## Prior Review Cross-Reference

| Review | State | Status |
|---|---|---|
| contract-verification-review | S3 | APPROVED |
| proof-review | S6 | REJECTED (3 LETHAL — all repaired) |
| test-suite-review | S9 | REJECTED (1 BLOCK_LOCAL — repaired) |
| formal-verification | S11 | PASS (264 tests, clippy clean) |

---

## Phase 1: Contract & Bead Parity — PASS

All 4 acceptance criteria verified:

**AC-1**: Author YAML no longer requires low-level slots/actions
- `lower_slot_reference` converts `$slot.N` and `$slots.N.P` to `LoadSlot(u16)` and `LoadAccessor(u16)` numeric handles. Author-facing API uses symbolic slot names; lowering produces numeric IR. (evidence: `expression_bytecode.rs`)

**AC-2**: Invalid references fail before runtime
- `validate_workflow_ast` runs before lowering. `lower_slot_reference` returns `CompileError::UnknownReferenceName` and `CompileError::UnknownReferenceRoot`. All 11 error variants in ERR-* taxonomy covered. (evidence: `references.rs`)

**AC-3**: Lowered IR preserves value/action/ref/taint semantics
- 32 taint preservation tests in `type_taint/tests.rs`. `lower_literal` preserves type metadata. `lower_accessor_reference` produces `AccessorProgram` entries. (evidence: `type_taint/tests.rs`)

**AC-4**: Runtime core receives numeric/handle data only
- `SlotCompiler` produces `CompiledNodeKind::LoadSlot(u16)` and `CompiledNodeKind::LoadAccessor(u16, AccessorProgram)`. No symbolic references leak past the lowering boundary. (evidence: `lib.rs` SlotCompiler)

All 32 contract clauses traced in `traceability-matrix.jsonl`. No orphan clauses.

---

## Phase 2: Farley Engineering Rigor — PASS

- **Function length < 25 lines**: `lower_slot_reference`, `lower_accessor_reference`, `compile_expr_to_bytecode`, `lower_steps_to_ir` all pass. (evidence: `expression_bytecode.rs`, `lib.rs`)
- **Parameter count ≤ 5**: No function exceeds limit. (evidence: `expression_bytecode.rs`, `lib.rs`)
- **Pure lowering / impure shell separation**: `lower_*` functions are pure; `SlotCompiler::build_parts` is impure shell. Correct layering. (evidence: `lib.rs`)
- **Tests assert behavior**: Tests verify `LoadSlot(7)` output not internal state. (evidence: `references/tests.rs`)

---

## Phase 3: Holzman Rust (The Big 6) — PASS

- **Make illegal states unrepresentable**: `CompileError` enum (11 variants), `ExprOp` enum, `WaitKind` enum. (evidence: `lib.rs`, `expression_bytecode.rs`)
- **Parse, don't validate**: `SlotCompiler::new` receives pre-validated `RefTables`; slot indices parsed to u16 at lowering boundary. (evidence: `expression_bytecode.rs`)
- **Types as documentation**: `LoadSlot(u16)` typed numeric index (not generic int). `AccessorProgram` explicit vector. (evidence: `lib.rs`)
- **Workflows**: `build_parts` explicit 4-phase pipeline: validate → lower actions → lower expressions → assemble. (evidence: `lib.rs` `SlotCompiler::build_parts`)
- **No panic vector**: No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg` in production code. (evidence: `clippy -D warnings` PASS)

---

## Phase 4: Ruthless Simplicity & DDD — PASS

- All fallible functions return `Result<T, CompileError>`. No `Option` as state machine. (evidence: `expression_bytecode.rs`)
- **CUPID**: Composable (lower_* compose), Predictable (pure/deterministic), Idiomatic (standard Rust `?`), Domain-based (`SlotCompiler`, `AccessorProgram`, `ConstValue`).

---

## Phase 5: The Bitter Truth — PASS

- No clever metaprogramming or abstract trait hierarchies. (evidence: codebase review)
- No over-engineered abstraction for single-use cases. (evidence: codebase review)
- Code is obvious on first reading. (evidence: codebase review)
- No YAGNI violations. (evidence: codebase review)

---

## Defect Summary

All LETHAL/MAJOR issues from prior reviews FIXED:

| ID | Severity | Description | Status |
|---|---|---|---|
| F-001 | LETHAL | `lower_slot_reference_for_testing` not exported | FIXED |
| F-002 | LETHAL | kani-harnesses not integrated | FIXED |
| F-003 | LETHAL | rust-verification-gauntlet.sh missing | FIXED |
| BLOCK-1 | BLOCK_LOCAL | Kani harnesses not in lib.rs | FIXED |
| F-004–F-008 | MAJOR | Harness issues | FIXED |

Fix evidence:
- `crates/vb_compile/src/kani/mod.rs`: all 5 modules declared
- `crates/vb_compile/src/lib.rs`: `#[cfg(kani)] pub mod kani;`
- `scripts/rust-verification-gauntlet.sh`: exists (5.5K, executable)

---

## GATE-VERIFY-FAST-001 Deferred Obligation — SATISFIED

Script exists. 264 tests pass. Clippy clean. Gauntlet is a moon-layer wrapper; primary evidence is `cargo test` + `clippy` output in `implementation.md`.

---

## Final Verdict

**STATUS: APPROVED**

All 5 phases pass. All LETHAL blockers repaired. 264 tests pass. Clippy clean. No defects found.
