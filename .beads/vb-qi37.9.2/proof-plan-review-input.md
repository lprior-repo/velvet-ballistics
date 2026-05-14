# Proof Plan Review Input — vb-qi37.9.2

## Context
- **Bead**: vb-qi37.9.2
- **Title**: expr: Execute F64 bytecode semantics
- **State**: 4 → review input for State 6 (proof-reviewer)
- **Reviewer task**: Determine if the planned verifier lanes and obligation matrix are adequate for the F64 bytecode semantics scope

## What Was Planned

### Verifier Lanes Chosen
- **Kani** (2 obligations): F64 add/sub/mul/div overflow → NonFiniteFloat, F64/0 → NonFiniteFloat (not DivisionByZero)
- **proptest** (7 obligations): FiniteF64 constructor, F64 arithmetic ops, F64 comparisons, stack bounds, I64 overflow
- **cargo careful** (1 obligation): runtime UB detection for safe Rust
- **clippy + build** (2 obligations): standard machine gates
- **waived** (1): FUZZ-CONST-001 — no harness, roundtrip tests compensate

### Verifier Lanes Rejected with Rationale
| Lane | Rejected | Reason |
|------|----------|--------|
| TLA+ | yes | No temporal/state-over-time behavior; pure deterministic computation |
| Verus | yes | No refinement types or type-state predicates; Kani+proptest sufficient |
| Flux | yes | No refinement predicates in F64 path |
| Loom | yes | No concurrency in F64 bytecode eval |
| Miri | yes (blocked) | `#![forbid(unsafe_code)]` on both crates — no unsafe code exists |

## Obligation Matrix Summary

### Kani Obligations
- **PO-001** (`KANI-F64-001`): F64 add/sub/mul/div produce non-finite exactly when FiniteF64::new rejects. Artifact: `crates/vb_expr/kani_f64_ops.rs`. Command: `cargo kani`.
- **PO-002** (`KANI-F64-002`): F64/0 → NonFiniteFloat, NOT DivisionByZero. Artifact: `crates/vb_expr/kani_f64_div.rs`. Command: `cargo kani`.

### proptest Obligations
- **PO-003** (`PROP-FINITE-001`): `FiniteF64::new` rejects NaN/Inf. Command: `cargo test -p vb_core finite_f64`.
- **PO-004** (`PROP-FINITE-002`): `FiniteF64::new` accepts all finite f64. Command: `cargo test -p vb_core finite_f64_accepts`.
- **PO-005** (`PROP-EVAL-F64-001`): eval_add_op IEEE 754 correctness. Command: `cargo test -p vb_expr f64`.
- **PO-006** (`PROP-EVAL-F64-002`): eval_sub_op IEEE 754 correctness. Command: `cargo test -p vb_expr f64`.
- **PO-007** (`PROP-EVAL-F64-003`): eval_mul_op IEEE 754 correctness. Command: `cargo test -p vb_expr f64`.
- **PO-008** (`PROP-EVAL-F64-004`): eval_div_op — F64/0 → NonFiniteFloat. Command: `cargo test -p vb_expr f64_div`.
- **PO-009** (`PROP-EVAL-F64-005`): eval_neg_op IEEE 754 negation. Command: `cargo test -p vb_expr f64`.
- **PO-010** (`PROP-EVAL-F64-006`): F64 comparisons — NaN → false. Command: `cargo test -p vb_expr f64`.
- **PO-011** (`PROP-STACK-001`): Stack overflow → StackOverflow error. Command: `cargo test -p vb_expr stack_overflow`.
- **PO-012** (`PROP-STACK-002`): I64 overflow → IntegerOverflow. Command: `cargo test -p vb_expr integer_overflow`.

### Other Obligations
- **PO-013** (`CAREFUL-001`): cargo careful on vb_expr. Command: `cargo careful test -p vb_expr`.
- **PO-014** (`CLIPPY-001`): clippy with -D warnings. Command: `cargo clippy -p vb_expr -p vb_core`.
- **PO-015** (`BUILD-001`): clean build. Command: `cargo build -p vb_expr -p vb_core`.

### Waived Obligations
- **WO-001** (`FUZZ-CONST-001`): fuzz deserialize boundary — no harness. Waived; roundtrip tests compensate.

## Critical Design Decisions to Verify
1. **F64/0 semantics**: F64/0 yields ±Inf → `FiniteF64::new(±Inf)` fails → `Err(NonFiniteFloat)`. This is intentional and distinct from I64/0 → `Err(DivisionByZero)`. The Kani proof must confirm this distinction.
2. **NaN comparison**: NaN > x, NaN >= x, NaN < x, NaN <= x all return `false`. proptest must cover this.
3. **No unsafe code**: `#![forbid(unsafe_code)]` on both crates — no UB path through unsafe.

## Reviewer Questions
1. Is Kani the correct lane for F64 overflow verification, or should this be pure proptest?
2. Is the waived FUZZ-CONST-001 compensating evidence (serde roundtrip) sufficient?
3. Is `cargo careful` an acceptable substitute for Miri given `forbid(unsafe_code)`?
4. Are the 19 obligations from the contract phase correctly split into 15 planned + 1 waived + 11 N/A?

## Risk Coverage Assessment
| Risk Tag | Coverage |
|----------|----------|
| user-visible behavior | Kani + proptest + clippy |
| missing functionality (fold.rs) | Noted, not covered — outside scope |
| parser/codec | Not covered — not F64 eval scope |
| performance (stack) | proptest (stack_overflow) |
| persistence (I64 overflow) | proptest (integer_overflow) |
