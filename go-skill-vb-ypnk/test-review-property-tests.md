# Test Plan Review: Property Tests (Mode 1 — Plan Inquisition)

## VERDICT: REJECTED

---

### Axis 1 — Contract Parity: MAJOR GAP

| Property | Behavior Count | BDD Scenario Count | Coverage |
|----------|---------------|-------------------|----------|
| constant_folding | 18 (CF-1..CF-18) | 7 | Partial |
| bytecode_ast_parity | 12 (BP-1..BP-12) | 5 | Partial |
| digest_stability | 14 (DS-1..DS-14) | 3 | Partial |
| layout_stability | 10 (LS-1..LS-10) | 2 | Partial |
| bound_enforcement | 11 (BE-1..BE-11) | 3 | Partial |
| for_each_ordering | 18 (FE-1..FE-18) | 2 | Partial |
| taint_propagation | 14 (TP-1..TP-14) | 3 | Partial |
| arithmetic_overflow | 13 (AO-1..AO-13) | 3 | Partial |
| concurrency_safety | 19 (CS-1..CS-19) | 3 | Partial |
| resource_budget | 11 (RB-1..RB-11) | 3 | Partial |
| **error_recovery** | **22 (ER-1..ER-22)** | **3** | **CRITICAL GAP** |

**Finding AX1-MAJOR-1 — error_recovery overcount:**
Exit criteria (line 929) claims: *"Every public API behavior has at least one BDD scenario (all 22 ER scenarios written)"*

Section 3.11 actually contains exactly **3** scenarios:
- `error_recovery_eval_div_by_zero_returns_error_not_panic`
- `error_recovery_stack_underflow_returns_error_not_panic`
- `error_recovery_integer_overflow_returns_error_not_panic`

22 claimed vs 3 written = **19 scenarios missing**. ER-2 (MissingOutputSlot), ER-3 (MissingNextStep), ER-10 (QueueFull), ER-11 (RunNotFound), ER-12 (RunAlreadyExists), ER-13 (ShutdownInProgress), ER-14 (DispatchFailed), ER-15 (UnknownAction), ER-16 (ConstantPoolOverflow), ER-17 (BytecodeTooLong), ER-18 (HelperArityMismatch), ER-20 (diagnostic_code), ER-21 (Workflow errors escape as typed), ER-22 (recovered state consistent) — **none have dedicated BDD scenarios**.

**Finding AX1-MAJOR-2 — Multiple properties underspecified:**
- BP (12 behaviors): BP-2 (postfix order), BP-5 (helper arity), BP-7 (text literal rejection), BP-10 (max_stack) have scenarios but the 5 scenarios don't exhaustively cover all 12 behaviors.
- DS (14 behaviors): Only 3 scenarios written. DS-4 (monotonicity), DS-5 through DS-11 (determinism of all key functions), DS-13 (digest mismatch rejection), DS-14 (stale digest rejection) lack dedicated scenarios.
- LS (10 behaviors): Only 2 scenarios written. LS-3 through LS-7 (deterministic layout of SystemStatusView, RunSummaryView, WorkflowGraphView, ActionDescriptionView), LS-9 (Box<[T]> slice serialization), LS-10 (enum variant ordering) lack scenarios.
- FE (18 behaviors): Only 2 scenarios written. FE-3 (non-list type check), FE-5 (count ≤ limit succeeds), FE-11-13 (executed counter), FE-14 (limit=0 on empty), FE-17 (single-item list), FE-18 (2-item exhaustion) lack scenarios.

The plan's exit criteria self-assertion at line 929 is **factually incorrect** for error_recovery and misleading for multiple other properties.

**AXIS 1 Assessment**: The plan describes behavior inventories but does not provide ≥1 BDD scenario per distinct behavior for any property except possibly `bound_enforcement`. The contract function list is not shown (the plan assumes `contract.md` is external), so direct parity checking is impossible, but the self-claimed coverage in the exit criteria is demonstrably false for ER.

---

### Axis 2 — Assertion Sharpness: MAJOR

**Finding AX2-MAJOR-1 — for_each_ordering: exact error variant asserted but resource field not verified:**

Section 3.6, line 388-390:
```
Then: Err(IterationLimitExceeded) is returned AND item_slot is not bound
```

Section 1.6 (FE-4) defines the error as `Err(IterationLimitExceeded)` without a resource field in the behavior inventory. However, the **RB behaviors** (RB-1, RB-2) and the **FE behaviors** (FE-4) specify:
- RB-1: `Err(IterationLimitExceeded)` from `for_each_start` with list.count > fanout_limit
- RB-2: list.count ≤ fanout_limit succeeds

The FE scenario at line 388-390 is testing the limit-exceeded path but **does not assert which limit was exceeded** (`for_each_limit` vs some other resource). Compare to section 3.10, line 456 which correctly asserts:
```
Err(EngineError::IterationLimitExceeded { resource: "for_each_limit" })
```

The FE scenario at 388-390 is weaker than the equivalent RB scenario at 456.

**Finding AX2-MINOR-1**: The plan's exit criteria (line 934) claims: *"No test asserts only is_ok() or is_err() without specifying the value"*. The FE scenario at line 390 asserts `Err(IterationLimitExceeded)` — the variant is specified, satisfying the letter of the law. However, the exit criteria claim is inconsistent with the actual sharpness difference between FE (line 388-390) and RB (line 456) scenarios for the same error class.

**Finding AX2-MINOR-2**: Section 3.7 (taint_propagation) line 409: `Then: Ok(()) is returned` — this is `is_ok()` with no inner value. However, for taint propagation, `Ok(())` is the semantically correct return when validation passes (no taint leak found). This is acceptable per the skill's intent: `is_ok()` alone is LETHAL only when the function returns a meaningful value that should be checked; here it returns unit.

---

### Axis 3 — Trophy Allocation: MAJOR

**Finding AX3-MAJOR-1 — bytecode_ast_parity: 12 behaviors, 3 proptest strategies:**
Section 4.2 lists only 3 named proptest strategies:
- `bytecode_parity_add`
- `bytecode_parity_nested_precedence`
- `bytecode_parity_constant_pool_determinism`

BP-2 (postfix order), BP-3 (unary not lowering), BP-4 (negation lowering), BP-5 (helper arity), BP-6 (literal emits LoadConst), BP-9 (bytecode evaluator parity with AST), BP-10 (max_stack reflection), BP-11 (BytecodeTooLong), BP-12 (InvalidReference) — **9 of 12 behaviors have no named proptest strategy**. The plan may generate additional cases from the `any::<ExprAst>()` strategy (line 537), but this is not explicit.

Per the skill: *"Any pure function with non-trivial input space and no proptest invariant = LETHAL"*. The absence of explicit invariants for BP-2, BP-3, BP-4, BP-5, BP-6, BP-9, BP-10, BP-11, BP-12 is a gap.

**Finding AX3-MINOR-1**: The plan claims 11 primary + 44 sub-invariants = 55 invariant assertions. This exceeds the 5× threshold if the crate has ≤11 pub fns. The claim is plausible but unverifiable from the plan alone — the actual `pub fn` count from `contract.md` is not provided.

**Finding AX3-MINOR-2**: The 11/11 unit allocation is appropriate (all properties are pure Calc-layer). No integration test gap.

**Finding AX3-MINOR-3**: 7 fuzz targets provided. This satisfies the "parser/deserializer with fuzz" requirement.

---

### Axis 4 — Boundary Completeness: PASS (with notes)

| Property | Min | Max | Min-1 | Max+1 | Empty | Overflow | Notes |
|----------|-----|-----|-------|-------|-------|----------|-------|
| constant_folding | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |
| bytecode_ast_parity | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |
| digest_stability | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Arbitrary covers many boundaries |
| bound_enforcement | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |
| for_each_ordering | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | limit=0 covered in RB scenarios |
| taint_propagation | ✓ | ✓ | ✗ | ✗ | ✓ | ✗ | Empty covered (TP-10), max depth not explicit |
| arithmetic_overflow | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |
| concurrency_safety | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |
| resource_budget | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |
| error_recovery | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |

**Finding AX4-MINOR-1**: taint_propagation (TP) — deeply nested composite (TP-8) implies boundary at some max depth, but no explicit "one-above-max-depth" case named.

**Finding AX4-MINOR-2**: arithmetic_overflow AO-11 (eval_helper_length text > i64::MAX chars) and AO-12 (eval_helper_count > i64::MAX) — the "exceeds i64::MAX" boundary is named but no specific "i64::MAX chars exactly" or "i64::MAX + 1" scenario.

---

### Axis 5 — Mutation Survivability: PASS

All 22 mutation checkpoints have corresponding test assertions:
- MC-1 through MC-5 (unchecked arithmetic): caught by AO/BE proptest with overflow inputs
- MC-6 (stack bound): caught by proptest with oversized program
- MC-7 (validate_op_count): caught by proptest with >256 ops
- MC-8 (constant index mismatch): caught by bytecode parity proptest
- MC-9, MC-10 (taint checks): caught by TP proptest
- MC-11 (eval_helper_sum overflow): caught by AO proptest
- MC-12 (ensure_slot_capacity): caught by ER proptest
- MC-13 (queue capacity): caught by CS proptest
- MC-14, MC-15 (limit boundary): caught by RB/FE proptest at exact boundary
- MC-16 (finish_stack panic): caught by ER proptest
- MC-17 (blake3 swap): caught by DS proptest
- MC-19 (FE join reverses): caught by FE proptest order preservation
- MC-20 (None return for refs): caught by CF anti-invariant
- MC-21 (helper arity): scenario exists (BP-5) though proptest anti-invariant not explicit
- MC-22 (blake3 replacement): caught by DS proptest

**Finding AX5-MINOR-1**: MC-21 (helper arity validation) has a BDD scenario (BP-5 behavior) but no explicit proptest anti-invariant named in section 4.2. The `bytecode_parity_constant_pool_determinism` strategy uses `any::<ExprAst>()` which should cover it, but the anti-invariant is not named as it is for other properties.

---

### Axis 6 — Evidence Plan Audit: PASS

- All BDD scenarios specify `Given/When/Then` with explicit preconditions
- Proptest strategies use bounded, reproducible inputs (e.g., `0u64..1000u64` for seq numbers, `1u8..10u8` for capacity)
- Fuzz corpus seeds explicitly list edge cases: i64::MIN/MAX, f64::NAN/INFINITY, empty strings, max-depth AST
- Kani harnesses use `kani::any()` per GOD RULES (line 736)
- No unbounded randomness without reproducibility

---

## LETHAL FINDINGS

**None reach LETHAL threshold in isolation**, but the **cumulative gap between claimed and actual coverage** (especially ER: 22 claimed vs 3 written) combined with **AX3-MAJOR-1** (9 of 12 BP behaviors lacking named proptest invariants) creates a pattern of **systematic overstatement**.

---

## MAJOR FINDINGS (2)

1. **AX1-MAJOR-1**: error_recovery claims 22 BDD scenarios written; section 3.11 contains exactly 3. Exit criteria is factually false.
2. **AX3-MAJOR-1**: bytecode_ast_parity has 12 behaviors but only 3 named proptest strategies; 9 behaviors lack explicit proptest invariants.

---

## MINOR FINDINGS (5)

1. **AX1-MAJOR-2**: BP (12 behaviors/5 scenarios), DS (14/3), LS (10/2), FE (18/2) — behavior-to-scenario ratio is low; not all behaviors have dedicated scenarios.
2. **AX2-MINOR-1**: Exit criteria claim of exact-value assertions is inconsistent with FE line 390 not asserting the resource field.
3. **AX4-MINOR-1**: TP-8 (deeply nested composite) — max depth boundary not explicitly named.
4. **AX4-MINOR-2**: AO-11/AO-12 — exact i64::MAX boundary for length/count not named.
5. **AX5-MINOR-1**: MC-21 (helper arity) — BDD scenario exists but no explicit proptest anti-invariant named.

---

## MANDATE

1. **error_recovery**: Write the missing 19 BDD scenarios, or reduce the behavior inventory to match the 3 scenarios actually written. Do not claim 22 scenarios when 3 exist.
2. **bytecode_ast_parity**: Add explicit proptest invariants for BP-2, BP-3, BP-4, BP-5, BP-6, BP-9, BP-10, BP-11, BP-12. At minimum, name which strategy covers each behavior.
3. **FE line 390**: Assert `Err(IterationLimitExceeded)` with the resource field, matching RB line 456's precision.
4. **Exit criteria line 929**: Correct the claim. Either all 22 ER scenarios exist, or the count must reflect reality.
5. Resubmit for full Mode 1 re-review.
