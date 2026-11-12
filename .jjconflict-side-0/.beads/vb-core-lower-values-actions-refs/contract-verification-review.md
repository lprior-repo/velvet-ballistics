# Contract Verification Review — vb-core-lower-values-actions-refs

**Bead**: `vb-core-lower-values-actions-refs`
**Workspace**: `/tmp/vb-ws/vb-core-lower-values-actions-refs`
**Reviewer**: contract-verification-reviewer skill
**Date**: 2026-05-15

---

## STATUS: APPROVED

---

## Files Reviewed

- `.beads/vb-core-lower-values-actions-refs/contract/contract.md` — EXISTS (10,382 bytes)
- `.beads/vb-core-lower-values-actions-refs/contract/tla-spec.md` — EXISTS (2,941 bytes)
- `.beads/vb-core-lower-values-actions-refs/contract/lean-contract.md` — EXISTS (3,320 bytes)
- `.beads/vb-core-lower-values-actions-refs/contract/verification-layers.md` — EXISTS (5,660 bytes)
- `.beads/vb-core-lower-values-actions-refs/proof-obligations.planned.jsonl` — EXISTS (11,731 bytes, VALID JSONL)
- `.beads/vb-core-lower-values-actions-refs/contract/traceability-matrix.jsonl` — EXISTS (5,084 bytes, VALID JSONL)

---

## Command Evidence

```bash
$ jq -c . .beads/vb-core-lower-values-actions-refs/proof-obligations.planned.jsonl > /dev/null && echo "proof-obligations: VALID JSONL"
proof-obligations: VALID JSONL

$ jq -c . .beads/vb-core-lower-values-actions-refs/contract/traceability-matrix.jsonl > /dev/null && echo "traceability-matrix: VALID JSONL"
traceability-matrix: VALID JSONL
```

---

## Mandatory Verification Gate

| Check | Result |
|---|---|
| `contract.md` exists and non-empty | PASS |
| `tla-spec.md` exists and non-empty | PASS |
| `lean-contract.md` exists and non-empty | PASS |
| `verification-layers.md` exists and non-empty | PASS |
| `proof-obligations.jsonl` valid JSONL | PASS |
| `traceability-matrix.jsonl` valid JSONL | PASS |
| All JSONL entries have required fields | PASS |

---

## 1. Contract Coverage Review

### Clause Traceability

**Contract clauses in `contract.md`**: PRE-001–PRE-005, POST-001–POST-009, INV-001–INV-007, ERR-* taxonomy (11 error variants), PERF-*.

**Coverage in `proof-obligations.jsonl`**: All 17 obligations trace to specific contract clauses.

**Coverage in `traceability-matrix.jsonl`**: All 32 contract clauses map to at least one test or proof obligation.

| Contract Clause | Obligation(s) | Covered? |
|---|---|---|
| PRE-001 (SlotCompiler::new) | UNIT-SLOT-COMPILER-001, KANI-SLOT-REF-001 | YES |
| PRE-002 (lower_slot_reference u16) | KANI-SLOT-REF-001 | YES |
| PRE-003 (lower_accessor_reference numeric) | KANI-ACCESSOR-REF-001 | YES |
| PRE-004 (compile_expr_to_bytecode pre-validated) | UNIT-EXPR-BYTESTACK-001, VERUS-EXPR-STACK-001 | YES |
| PRE-005 (push_constant ConstValue only) | UNIT-LOWER-DO-001, KANI-CONSTANT-POOL-001 | YES |
| POST-001 (LoadSlot correct) | KANI-SLOT-REF-001 | YES |
| POST-002 (LoadAccessor correct) | KANI-ACCESSOR-REF-001 | YES |
| POST-003 (bytecode bounds) | KANI-EXPR-BYTECODE-001 | YES |
| POST-004 (single-stack-result) | UNIT-EXPR-BYTESTACK-001 | YES |
| POST-005 (constant pool) | KANI-CONSTANT-POOL-001 | YES |
| POST-006 (slot_count) | UNIT-SLOT-COMPILER-001 | YES |
| POST-007 (build_parts) | UNIT-BUILD-PARTS-001 | YES |
| POST-008 (taint preservation) | WAIVER | YES (waived) |
| POST-009 (validate before CompiledWorkflow) | POST-009-VALIDATE-001 | YES |
| INV-001 (max_slot tracking) | VERUS-SLOT-MAX-001, UNIT-SLOT-COMPILER-001 | YES |
| INV-002 (record_slot per slot) | UNIT-SLOT-COMPILER-001 | YES |
| INV-003 (StepIdx in bounds) | UNIT-LOWER-DO-001 | YES |
| INV-004 (bytecode stack safety) | VERUS-EXPR-STACK-001, KANI-EXPR-BYTECODE-001 | YES |
| INV-005 (numeric accessor paths) | KANI-ACCESSOR-REF-001, UNIT-ACCESSOR-REF-001 | YES |
| INV-006 (order-preserving) | INV-006-ORDER-001 | YES |
| INV-007 (unique node.id) | INV-007-NODEDUP-001 | YES |
| ERR::UnknownReferenceName | ERR-TAXONOMY-001, KANI-SLOT-REF-001 | YES |
| ERR::UnknownReferenceRoot | ERR-TAXONOMY-001 | YES |
| ERR::UnsupportedAccessorReference | ERR-TAXONOMY-001, UNIT-ACCESSOR-REF-001 | YES |
| ERR::ExpressionLoweringUnsupported | ERR-TAXONOMY-001 | YES |
| ERR::ExpressionHelperArity | ERR-TAXONOMY-001 | YES |
| ERR::ExpressionStackOverflow | VERUS-EXPR-STACK-001, KANI-EXPR-BYTECODE-001 | YES |
| ERR::ConstOutOfBounds | KANI-CONSTANT-POOL-001 | YES |
| ERR::SlotIndexOutOfRange | VERUS-SLOT-MAX-001 | YES |
| ERR::SecretTaintLeak | WAIVER | YES (waived) |
| ERR::IdempotencyViolation | STATIC-LINT-001 | YES |
| PERF::* | WAIVER | YES (waived) |

**Result**: All contract clauses are traced. No orphan clauses.

---

## 2. Verification Layer Fit Review

### Verus Obligations

| Obligation | Clause | Layer Fit | Waiver? |
|---|---|---|---|
| VERUS-EXPR-STACK-001 | INV-004 | Verus appropriate for pure integer stack effect | WAIVED (Verus not installed) |
| VERUS-SLOT-MAX-001 | INV-001 | Verus appropriate for pure max function | WAIVED (Verus not installed) |

**Verus waiver quality**: Both WAIVERS correctly identify `blocked_tooling` reason, compensating evidence (Kani + proptest), owner, and expiry. Waivers are **VALID**.

### Kani Obligations

| Obligation | Clause | Layer Fit | Command Valid? |
|---|---|---|---|
| KANI-EXPR-BYTECODE-001 | POST-003 | Kani appropriate for bounded bytecode overflow | `cargo kani --package vb_compile --harness` — YES |
| KANI-ACCESSOR-REF-001 | POST-002 | Kani appropriate for u16 index exhaust | `cargo kani --package vb_compile --harness` — YES |
| KANI-SLOT-REF-001 | POST-001 | Kani appropriate for u16 index exhaust | `cargo kani --package vb_compile --harness` — YES |
| KANI-CONSTANT-POOL-001 | POST-005 | Kani appropriate for u16::MAX exhaust | `cargo kani --package vb_compile --harness` — YES |
| INV-007-NODEDUP-001 | INV-007 | Kani appropriate for StepIdx uniqueness | `cargo kani --package vb_compile --harness` — YES |

**Note**: Kani harnesses are NOT currently integrated into the `vb_compile` crate (see proof-review.md BLOCKER-2). The layer assignment is correct; the integration is broken.

### Proptest/Unit Test Obligations

| Obligation | Clause | Layer Fit |
|---|---|---|
| UNIT-EXPR-BYTESTACK-001 | INV-004 | Correct — deterministic bytecode properties |
| UNIT-SLOT-COMPILER-001 | INV-001, INV-002 | Correct — deterministic data structure |
| UNIT-ACCESSOR-REF-001 | POST-002, INV-005 | Correct — parse predicate |
| ERR-TAXONOMY-001 | ERR-* | Correct — each error variant |
| UNIT-LOWER-DO-001 | PRE-005, INV-003 | Correct — lowering correctness |
| UNIT-BUILD-PARTS-001 | POST-007 | Correct — WorkflowParts construction |
| INV-006-ORDER-001 | INV-006 | Correct — order preservation |
| POST-009-VALIDATE-001 | POST-009 | Correct — validation call |

### Static Scan Obligation

| Obligation | Command | Source-only? |
|---|---|---|
| STATIC-LINT-001 | `cargo clippy -p vb_compile --lib -- -D warnings -A unsafe_code` | YES — targets production source only |

---

## 3. TLA+, Verus, and Theorem Scope Review

### TLA+

`tla-spec.md` correctly identifies **no TLA+-owned clauses**. Rationale is sound: lowering is a pure function `WorkflowAst → WorkflowParts` with no temporal operators, liveness, fairness, state machines, or concurrent processes.

**TLA+ non-applicability**: APPROVED.

### Verus

`lean-contract.md` correctly identifies Verus as appropriate for:
- INV-004 (expression bytecode stack effect — pure integer recurrence)
- INV-001 (max_slot tracking — pure integer max function)
- INV-005 (numeric-only accessor paths — parse predicate)

Both Verus obligations are **WAIVED** due to `blocked_tooling` (Verus not installed in CI). Compensating evidence from Kani + proptest is adequate per waiver.

**Verus scope**: APPROVED with valid waivers.

### Theorem (Lean/Aeneas/Hax)

`lean-contract.md` correctly identifies **no theorem-owned clauses**. Rationale: all properties are expressible as Verus specs/proofs or unit-testable data structure properties. No algebraic state transitions, no protocol lattices, no refinement chains beyond Verus scope.

**Lean/Aeneas/Hax non-applicability**: APPROVED.

---

## 4. Executable Obligation Shape Review

### proof-obligations.jsonl Completeness

All 17 entries checked for required fields:

| Field | Present in all? |
|---|---|
| `id` | YES |
| `contract_clause` | YES |
| `target` | YES |
| `claim` | YES |
| `layer` | YES |
| `checker` | YES |
| `command` | YES |
| `evidence` | YES |
| `expected_evidence` | YES |
| `risk` | YES |
| `scope` | YES |
| `required` | YES |
| `mode` | YES |
| `owner_state` | YES |
| `rerun_from` | YES |
| `status` | YES (all `planned` or `blocked_tooling`) |

**Result**: All entries are complete and well-formed.

### Status Values

| Obligation | Status | Valid? |
|---|---|---|
| VERUS-EXPR-STACK-001 | `blocked_tooling` | YES (waived) |
| VERUS-SLOT-MAX-001 | `blocked_tooling` | YES (waived) |
| KANI-EXPR-BYTECODE-001 | `planned` | YES |
| KANI-ACCESSOR-REF-001 | `planned` | YES |
| KANI-SLOT-REF-001 | `planned` | YES |
| KANI-CONSTANT-POOL-001 | `planned` | YES |
| INV-007-NODEDUP-001 | `planned` | YES |
| All unit test obligations | `planned` | YES |
| STATIC-LINT-001 | `planned` | YES |
| GATE-VERIFY-FAST-001 | `planned` | YES |

---

## 5. Waiver Quality Review

### WAIVER-VERUS-EXPR-STACK (VERUS-EXPR-STACK-001)

| Field | Value | Valid? |
|---|---|---|
| Obligation waived | VERUS-EXPR-STACK-001 | YES |
| Verification layer | verus | YES |
| Reason | Verus toolchain not installed | YES |
| Compensating evidence | KANI-EXPR-BYTECODE-001 + UNIT-EXPR-BYTESTACK-001 | YES |
| Owner | proof-planner | YES |
| Expiry | Until Verus installed in CI | YES |
| Follow-up | If cargo verus --version succeeds, re-run proof-planner | YES |

### WAIVER-VERUS-SLOT-MAX (VERUS-SLOT-MAX-001)

| Field | Value | Valid? |
|---|---|---|
| Obligation waived | VERUS-SLOT-MAX-001 | YES |
| Verification layer | verus | YES |
| Reason | Verus toolchain not installed | YES |
| Compensating evidence | KANI-SLOT-REF-001 + UNIT-SLOT-COMPILER-001 | YES |
| Owner | proof-planner | YES |
| Expiry | Until Verus installed in CI | YES |
| Follow-up | Same as WAIVER-VERUS-EXPR-STACK | YES |

**Waiver quality**: Both waivers are complete and valid per the skill's rule.

---

## 6. Error Taxonomy Coverage

All 11 error variants in `contract.md` error taxonomy have corresponding test or proof coverage:

| Error Variant | Coverage |
|---|---|
| `UnknownReferenceName` | ERR-TAXONOMY-001 + KANI-SLOT-REF-001 |
| `UnknownReferenceRoot` | ERR-TAXONOMY-001 |
| `UnsupportedAccessorReference` | ERR-TAXONOMY-001 + UNIT-ACCESSOR-REF-001 |
| `ExpressionLoweringUnsupported` | ERR-TAXONOMY-001 |
| `ExpressionHelperArity` | ERR-TAXONOMY-001 |
| `ExpressionStackOverflow` | VERUS-EXPR-STACK-001 + KANI-EXPR-BYTECODE-001 |
| `ConstOutOfBounds` | KANI-CONSTANT-POOL-001 |
| `SlotIndexOutOfRange` | VERUS-SLOT-MAX-001 + KANI-SLOT-REF-001 |
| `SecretTaintLeak` | WAIVER (order-guaranteed pipeline) |
| `IdempotencyViolation` | STATIC-LINT-001 |
| `Validation(...)` | POST-009-VALIDATE-001 |

**Result**: Complete error taxonomy coverage.

---

## Coverage Decision

| Dimension | Decision |
|---|---|
| Contract clauses traced | ALL 32 traced |
| TLA+-owned clauses covered | NONE (correctly identified as N/A) |
| Verus-owned clauses covered | INV-004, INV-001, INV-005 (WAIVED) |
| Theorem-owned clauses covered | NONE (correctly identified as N/A) |
| Proof obligations traced | ALL 17 obligations |
| TLA+ scope valid | YES — lowering is pure function |
| Verus scope valid | YES — pure integer properties |
| Lean/Aeneas/Hax scope valid | YES — no theorem kernels |
| Waivers valid | YES — both Verus waivers complete and sound |
| Error taxonomy covered | YES — all 11 variants |
| Layer assignments fit | YES — Kani/proptest/Verus/static-scan all appropriate |
| Command specificity | YES — all commands name package, harness, or target |
| Source-lint targets production only | YES — STATIC-LINT-001 targets `vb_compile --lib` |

---

## Findings

No rejections. Contract is comprehensive and well-formed.

**Note for downstream**: The contract verification review is APPROVED. However, the **proof-review** (separate artifact) found 3 LETHAL blockers and 5 MAJOR issues in the proof-writer artifacts (kani harnesses not integrated, missing helper function, harness logic bugs). The contract itself is sound — the implementation of the proof obligations needs repair.
