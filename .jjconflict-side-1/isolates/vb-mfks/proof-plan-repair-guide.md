# Proof Plan Repair Guide — vb-mfks

## Purpose
This guide provides concrete repair actions to resolve the 6 blocking issues identified in the proof plan review.

---

## Issue 1: Internal Inconsistency — Violation Count

**Problem**: proof-strategy.md title says "14 violations" but coverage matrix shows "26 planned violations"

**Repair Actions**:
1. Count actual violations in source files:
   - kani_trace_ring.rs: 2 hardcoded StepIdx/SlotIdx + 1 hardcoded array = 3
   - kani_admission_store.rs: 2 vacuous assertions = 2
   - kani_gate_08_structural.rs: 5 hardcoded WorkflowParts + 5 no-result = 10
   - kani_capability_harnesses.rs: 10 hardcoded capability names = 10
   - **Total: 25 violations**
2. Update proof-strategy.md title: "Fix 25 vb_runtime/vb_validate Kani GOD RULE violations"
3. Update proof-coverage-matrix.md summary row to match

---

## Issue 2: PO-005 Harness Range Error

**Problem**: States "7 hardcoded WorkflowParts harnesses (H9-H14)" but only 5 are actually hardcoded

**Source Analysis**:
| Harness | Line | Status |
|---------|------|--------|
| H9: kani_gate_08_empty_nodes_valid_accessors_pass | 200 | HARDCODED ✓ |
| H10: kani_gate_08_expressions_with_accessor_refs | 228 | HARDCODED ✓ |
| H11: kani_gate_08_mixed_accessor_paths | 286 | HARDCODED ✓ |
| H12: kani_gate_08_all_node_kinds_no_panic | 352 | uses kani::any() ✗ |
| H13: kani_gate_08_constants_with_symbols | 363 | HARDCODED ✓ |
| H14: kani_gate_08_many_accessors_varied_depths | 414 | HARDCODED ✓ |

**Repair Actions**:
1. Update PO-005 to list exactly: H9, H10, H11, H13, H14 (5 harnesses)
2. If PO-005 should also cover H5/H7/H8 (which use kani::any() but have hardcoded struct literals in other functions), clarify scope
3. Update vld-005 rationale: "5 hardcoded WorkflowParts harnesses (H9-H11, H13-H14) must use kani::any()"
4. Remove all references to "7" for hardcoded WorkflowParts count

---

## Issue 3: Vague Language in vld-005

**Problem**: Contains "possibly one more" which is unacceptable ambiguity

**Repair Actions**:
1. Remove "possibly one more" from vld-005 rationale
2. After verifying source, state exact harness count and names
3. If uncertainty exists, mark as "investigation required" rather than using vague qualifiers

---

## Issue 4: Missing contract-spec.md

**Problem**: contract-spec.md is listed in review checklist but absent from isolate

**Repair Actions**:
1. If contract-spec.md exists in source checkout, copy to isolate
2. If no contract-spec.md exists for these modules, document this explicitly:
   ```
   contract-spec.md: NOT APPLICABLE — vb_runtime/vb_validate Kani harnesses 
   verify Send+Sync hygiene and structural bounds, not domain contracts
   ```
3. Update proof-strategy.md non-applicable lanes section to explain why no contract spec is needed

---

## Issue 5: Circular State References

**Problem**: owner_state: 5 and rerun_from: 5 on planned obligations

**Repair Actions**:
1. Change all `rerun_from: 5` to `rerun_from: null` in proof-obligations.planned.jsonl
2. Alternatively, clarify that `rerun_from` indicates the state from which a *failure* can be rerun, not the current state
3. Lane decisions should use `owner_state: null` for not_applicable entries (currently set to null — correct)

---

## Issue 6: Ambiguous Harness Target in PO-007

**Problem**: `--harness check_capability_harness` matches only 1 of 14 capability-related harnesses

**Source Analysis of Violations in kani_capability_harnesses.rs**:
| Harness | Line | Hardcoded Names |
|---------|------|-----------------|
| strict_admission_invalid_artifact_cases_reject | 135 | "network" |
| strict_admission_invalid_capability_rejects | 168 | "network" |
| strict_admission_valid_artifact_admits | 225 | "network" |
| check_capability_grants_exact_match | 275 | "action" |
| check_capability_action_match_name_grants | 288 | "network" |
| check_capability_action_match_name_denies | 299 | "secrets", "network" |
| check_capability_action_mismatch_name_grants | 314 | "network" |
| check_capability_action_mismatch_name_denies | 329 | "secrets", "network" |
| check_capability_hierarchical_rejects_subpath | 344 | "network.api", "network" |
| check_capability_partial_segment_rejected | 359 | "network", "net" |

**Repair Actions**:
1. Split PO-007 into multiple obligations by harness
2. Or use cargo kani with glob pattern: `--harness 'check_capability_'`
3. Or list all 10 violating harnesses explicitly in a single PO with compound command

---

## Summary of Required Changes

| File | Change Required |
|------|----------------|
| proof-strategy.md | Fix title count (14 → 25), update Obligations Summary table |
| proof-obligations.planned.jsonl | Fix PO-005 harness list, set rerun_from: null, fix PO-007 command |
| verifier-lane-decisions.jsonl | Remove vague "possibly one more" from vld-005, fix harness counts |
| proof-coverage-matrix.md | Update summary to match actual counts |
| contract-spec.md | Provide artifact or document N/A |
| proof-plan-review.md | This document — already written |

---

**Priority Order**: Fix Issue 2 (PO-005) and Issue 3 (vague language) first, as they affect the core obligation definitions. Then address Issue 1 (count mismatch), Issue 6 (PO-007 command), Issue 5 (state semantics), and Issue 4 (contract-spec) in any order.
