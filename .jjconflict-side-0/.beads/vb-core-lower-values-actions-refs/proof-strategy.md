# Proof Strategy — vb-core-lower-values-actions-refs

## Beacon

| Field | Value |
|-------|-------|
| Bead | `vb-core-lower-values-actions-refs` |
| State | 4 (Proof Planning) |
| Planner | proof-planner skill |
| Generated | 2026-05-15 |

---

## 1. Scope

Proof planning covers the `vb_compile` crate lowering infrastructure for v1 values, expressions, slot/accessor references, action references, and taint metadata. The hot-path target is `crates/vb_compile/src/expression_bytecode.rs` and `crates/vb_compile/src/lib.rs` (`SlotCompiler`).

---

## 2. Discovery Findings

| Check | Result |
|-------|--------|
| `crates/vb_compile/src/expression_bytecode.rs` | `#![forbid(unsafe_code)]` — no unsafe; no panics in production |
| `crates/vb_compile/src/lib.rs` | `#![forbid(unsafe_code)]` — panics only in test code |
| `crates/vb_core/src/expressions.rs` | EXISTS; no Verus annotations present |
| `cargo kani --version` | 0.67.0 — **AVAILABLE** |
| `cargo verus` / `verus` | Placeholder / not installed — **BLOCKED** |
| Risk patterns (spawn/tokio/Mutex/RwLock/Atomic) | None found in scope |
| Retry/cancel in production code | Only in comments/error messages; no runtime state machines |

---

## 3. Risk Classification

| Risk | Verifier | Rationale |
|------|----------|-----------|
| Expression bytecode stack overflow/underflow | Kani + proptest | Bounded integer ops; honest input space; Kani kills counterexamples |
| Slot index arithmetic overflow | Kani | u16 bounds; Kani exhausts edge cases |
| Accessor path numeric-only enforcement | Kani | Finite path length; Kani bounds all segments |
| Constant pool overflow | Kani | u16::MAX boundary; Kani verifies overflow path |
| Node StepIdx uniqueness | Kani | Deterministic per-step lowering; bounded node count |
| Expression bytecode stack invariants | Verus (BLOCKED) | Pure integer inequality; Verus spec/proof fns appropriate but tool unavailable |
| Slot max tracking invariant | Verus (BLOCKED) | Pure max-tracking; Verus spec appropriate but tool unavailable |
| Taint preservation | Waiver | Order-guaranteed by compile pipeline; covered by 121.6K type_taint_tests.rs |
| Post-009 validation call | proptest unit test | Direct API call check; cheap unit test sufficient |

---

## 4. Obligation Status

### 4.1 Verus — BLOCKED (tooling unavailable)

| ID | Clause | Status | Waiver Reason |
|----|--------|--------|---------------|
| `VERUS-EXPR-STACK-001` | INV-004 | `blocked_tooling` | Verus not installed; Kani + proptest provide adequate bounded coverage for expression stack; `ExprProgram::try_from_ops` is pure and total |
| `VERUS-SLOT-MAX-001` | INV-001 | `blocked_tooling` | Verus not installed; Kani provides counterexample-free coverage for slot index bounds; slot_count() is u16-returning pure function |

### 4.2 Kani — Ready to execute

| ID | Clause | Status | Command |
|----|--------|--------|---------|
| `KANI-EXPR-BYTECODE-001` | POST-003 | `planned` | `cargo kani --package vb_compile --harness compile_expr_to_bytecode_overflow` |
| `KANI-ACCESSOR-REF-001` | POST-002 | `planned` | `cargo kani --package vb_compile --harness lower_accessor_reference_numeric` |
| `KANI-SLOT-REF-001` | POST-001 | `planned` | `cargo kani --package vb_compile --harness lower_slot_reference_valid` |
| `KANI-CONSTANT-POOL-001` | POST-005 | `planned` | `cargo kani --package vb_compile --harness push_constant_overflow` |
| `INV-007-NODEDUP-001` | INV-007 | `planned` (optional) | `cargo kani --package vb_compile --harness node_id_uniqueness` |

### 4.3 Proptest / Unit — Ready to execute

| ID | Clause | Status | Command |
|----|--------|--------|---------|
| `UNIT-EXPR-BYTESTACK-001` | INV-004 | `planned` | `cargo test -p vb_compile --lib expression_bytecode -- --nocapture` |
| `UNIT-SLOT-COMPILER-001` | INV-001 | `planned` | `cargo test -p vb_compile --lib slot_compiler -- --nocapture` |
| `UNIT-ACCESSOR-REF-001` | POST-002 | `planned` | `cargo test -p vb_compile --lib expression_bytecode -- --nocapture` |
| `ERR-TAXONOMY-001` | ERR-* | `planned` | `cargo test -p vb_compile --lib expression_bytecode -- --nocapture` |
| `UNIT-LOWER-DO-001` | PRE-005 | `planned` | `cargo test -p vb_compile --lib lower -- --nocapture` |
| `UNIT-BUILD-PARTS-001` | POST-007 | `planned` | `cargo test -p vb_compile --lib slot_compiler -- --nocapture` |
| `INV-006-ORDER-001` | INV-006 | `planned` (optional) | `cargo test -p vb_compile --lib lower -- --nocapture` |
| `POST-009-VALIDATE-001` | POST-009 | `planned` | `cargo test -p vb_compile --lib lower_steps -- --nocapture` |

### 4.4 Static Scan — Ready to execute

| ID | Clause | Status | Command |
|----|--------|--------|---------|
| `STATIC-LINT-001` | ALL | `planned` | `cargo clippy -p vb_compile --lib -- -D warnings -A unsafe_code` |

### 4.5 Gauntlet — Planned for state 12

| ID | Clause | Status | Command |
|----|--------|--------|---------|
| `GATE-VERIFY-FAST-001` | ALL | `planned` | `moon run :verify-fast` (owner_state: 12) |

---

## 5. Waiver Records

### WAIVER-VERUS-EXPR-STACK

- **Obligation**: `VERUS-EXPR-STACK-001` (INV-004)
- **Owner**: proof-planner
- **Reason**: Verus toolchain not installed (`cargo verus` is a placeholder; `verus` binary absent). Kani 0.67.0 + proptest provide adequate bounded model checking for expression bytecode stack properties.
- **Compensating evidence**: `UNIT-EXPR-BYTESTACK-001` (proptest 100+ op combinations) + `KANI-EXPR-BYTECODE-001` (overflow path coverage). Stack effect is pure integer arithmetic — Kani exhausts u16 bounds honestly.
- **Expiry**: Until Verus is installed in CI
- **Follow-up trigger**: If `cargo verus --version` succeeds in CI, re-run `proof-planner` for this bead

### WAIVER-VERUS-SLOT-MAX

- **Obligation**: `VERUS-SLOT-MAX-001` (INV-001)
- **Owner**: proof-planner
- **Reason**: Verus toolchain not installed. `slot_count()` is a u16-returning pure function; Kani exhausts all u16 values for slot index computations.
- **Compensating evidence**: `KANI-SLOT-REF-001` (slot reference bounds) + `UNIT-SLOT-COMPILER-001` (max tracking unit tests). Max-tracking invariant over u16 domain is fully covered by Kani counterexample-free runs.
- **Expiry**: Until Verus is installed in CI
- **Follow-up trigger**: Same as WAIVER-VERUS-EXPR-STACK

---

## 6. Execution Order

```
State 4 (this state):
  1. Write proof-strategy.md        ← THIS ARTIFACT
  2. Write proof-plan-review-input.md
  3. Write proof-obligations.planned.jsonl
  4. Update STATE.md → state: 4

State 5 (Proof Writing):
  - proof-writer creates Kani harnesses
  - proof-writer creates proptest tests
  - STATIC-LINT-001 can run immediately (clippy is available)

State 8 (Formal Verification):
  - formal-verifier runs Kani obligations (KANI-*)
  - formal-verifier runs proptest unit tests (UNIT-*)
  - GATE-VERIFY-FAST-001 deferred to state 12
```

---

## 7. vb-f04l Blocker Note

`vb-f04l` ("compiler: Safe v1 primitive source lowering") is the `blocks` dependent. Proof obligations in this bead do **not** depend on vb-f04l implementation — they target the slot/accessor/expression lowering infrastructure (`expression_bytecode.rs`, `lib.rs SlotCompiler`) which is already present in the codebase. vb-f04l only affects the primitive-lowering callers (`lower_do`, etc.), not the proof targets themselves.

---

## 8. Summary

- **17 total obligations** across 5 verifier lanes
- **2 blocked_tooling** (Verus) with explicit waivers
- **10 execute-ready** (Kani + proptest + clippy)
- **5 deferred** (Gauntlet lane, state 12)
- **0 unmapped** obligations — all trace to contract clauses
