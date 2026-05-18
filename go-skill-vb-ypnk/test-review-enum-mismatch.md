# Test Plan Review: MAJOR-6 — SideEffect/RetrySafety Enum Mismatch

**Bead**: MAJOR-6
**Mode**: Plan Inquisition (Mode 1)
**Date**: 2026-05-17

---

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity: **FAIL**

**Finding LETHAL**: Internal logical contradiction in `retry_safety_exhaustive_match_against_master_plan`.

The scenario asserts two mutually exclusive conditions simultaneously:
1. "No additional variants exist beyond the master plan set" (implies `Unsafe` should not exist)
2. Enum Comparison Matrix documents that `Unsafe` IS present in implementation and is EXTRA

The test as written would ALWAYS FAIL — even after any "fix", the contradiction means the test cannot pass. This is not a test plan that validates correct behavior; it is a document of broken behavior that cannot validate its own repair.

**Finding MAJOR**: The plan documents mismatches but provides NO remediation path. The scenarios describe WHAT IS WRONG but not WHAT SHOULD BE. After implementation is changed to match Section 65, there is no test validating that the correct variants (`Pure`, `LocalRead`, `NotRetrySafe`, `Unknown`, etc.) actually exist in the FIXED implementation — only that the BROKEN implementation was broken.

---

## Axis 2 — Assertion Sharpness: **FAIL**

**Finding MAJOR**: Internal contradiction in SideEffect variant assertions.

`side_effect_none_variant_matches` scenario:
```
Then: The variant exists with the exact name "None"
```

But the Enum Comparison Matrix (Section 4) explicitly marks `None` as:
```
— | `None` | ❌ EXTRA (not in master plan)
```

The plan simultaneously asserts "None variant matches" AND documents that `None` is an extra variant not in the master plan. These are contradictory.

---

## Axis 3 — Trophy Allocation: **MAJOR**

**Finding MAJOR**: 13 tests for 2 behaviors (6.5×) is technically above the 5× threshold, but the tests are documenting broken state, not validating fixed state. The ratio is misleading — these tests will fail before the fix and (due to logical contradictions) may fail after the fix.

Additionally: the plan states "0 proptest invariants" and "0 fuzz targets" with rationale "no property space" — this is appropriate for static enum comparison but leaves the mutation kill rate dependent entirely on exhaustive match tests with internal contradictions.

---

## Axis 4 — Boundary Completeness: **MINOR**

- Variant count coverage: ✓
- Naming coverage: ✓  
- Discriminant uniqueness: ✓
- No extras assertion: ✓ (but internally contradictory)

**MINOR (1/5 threshold)**: No test for duplicate discriminant values (two variants with same `#[repr(u8)]`). Rust compiler prevents this, so functionally acceptable.

---

## Axis 5 — Mutation Survivability: **MAJOR**

Mutation scenarios correctly identified:
| Mutation | Catching test |
|----------|---------------|
| SideEffect variant removed | `side_effect_variants_match_section_65` ✓ |
| RetrySafety variant removed | `retry_safety_variants_match_section_65` ✓ |
| New SideEffect variant added | `side_effect_exhaustive_match_against_master_plan` ✓ |
| New RetrySafety variant added | `retry_safety_exhaustive_match_against_master_plan` ✓ |

**Finding MAJOR**: The mutation table identifies WHICH tests would catch add/remove mutations, but the tests themselves are logically broken (see Axis 1). The mutation coverage is theoretical, not practical.

---

## Axis 6 — Evidence Plan Audit: **MAJOR**

**Finding MAJOR — Unresolved contract ambiguity (Open Question 1, Section 11)**:

> "Should the fix align the implementation to the master plan names..., or should the master plan be updated to match the existing implementation?"

This is not an open question for a test plan. A test plan must have a definitive expected state. Without knowing whether the FIX means "implementation matches master plan" or "master plan matches implementation", no test can be written that validates correct behavior.

The plan also leaves these unresolved:
1. Semantic equivalence of `Unsafe` vs `NotRetrySafe` — are these the same or different?
2. Whether `Unsafe` should be preserved or removed in the fix

---

## Summary of Lethal/Major Findings

### LETHAL (1 — any single = REJECTED)

1. **`retry_safety_exhaustive_match_against_master_plan`**: Internally contradictory assertions. Asserts "no additional variants" while documenting that `Unsafe` IS an additional variant. Test cannot pass after any fix.

### MAJOR (3 — threshold for rejection)

1. **No remediation validation**: Plan documents current broken state but provides no test validating the FIXED state
2. **Internal contradiction in SideEffect**: `side_effect_none_variant_matches` asserts "None matches" while Matrix shows `None` is extra
3. **Unresolved contract ambiguity**: Open Question 1 (implementation vs master plan as source of truth) makes the test target undefined

---

## Mandatory Changes Before Resubmission

1. **Resolve the contradiction**: Either remove `Unsafe` from implementation (and update tests to not assert it exists) OR update master plan to include `Unsafe` (and update exhaustive match assertion)
2. **Define the fix target explicitly**: State in the plan header whether the implementation must be changed to match master plan, or master plan must be updated to match implementation
3. **Add validation scenarios**: After the fix is applied, there must be tests that assert the CORRECT state (e.g., "SideEffect has Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell") not just that the current wrong state is wrong
4. **Clarify semantic equivalence**: Explicitly state whether `Safe` = `Idempotent`, `KeyRequired` = `RequiresIdempotencyKey`, and whether `Unsafe` = `NotRetrySafe` or `Unknown` or neither
5. **Remove internal contradictions**: Every scenario's "Then" must be achievable by exactly one state of the world

---

## What Is Valid in This Plan

- The Enum Comparison Matrix (Section 4) is a thorough and accurate diagnosis
- The gap analysis (Section 5) correctly identifies missing, extra, and mismatched variants  
- The mutation checkpoint identification (Section 9) correctly maps mutations to catching tests
- Test function names (Section 10) are well-chosen and descriptive

The diagnosis is solid. The test scenarios to validate the fix are not.
