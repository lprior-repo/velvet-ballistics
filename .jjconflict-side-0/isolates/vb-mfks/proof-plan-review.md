# Proof Plan Review — vb-mfks

## STATUS: REJECTED

## Reviewer
- **Role**: proof-plan-reviewer
- **Invocation ID**: ppr-vb-mfks-001
- **Planner Invocation ID**: unknown (not recorded in artifacts)

## Lanes Reviewed
- **Kani** (primary/only lane for execution)
- TLA+, Verus, Flux, Loom, Miri, proptest, cargo-fuzz (all correctly marked not_applicable)

## Blocking Issues

### 1. Internal Inconsistency: Violation Count Mismatch
- **proof-strategy.md** title: "Fix 14 vb_runtime/vb_validate Kani GOD RULE violations"
- **proof-coverage-matrix.md** summary: "26 planned violations"
- **Actual source analysis**: 17 hardcoded + 5 no-result + 2 vacuous = 24 violations (some overlap)

**Required fix**: Reconcile the violation count. The proof strategy must accurately reflect the actual number of violations being addressed.

### 2. PO-005: Incorrect Harnesses Range
- **Artifact**: proof-obligations.planned.jsonl, PO-005
- **Issue**: States "7 hardcoded WorkflowParts harnesses (H9-H14)" but H9-H14 is only **6 harnesses** (H9, H10, H11, H12, H13, H14)
- **Source verification**:
  - H9 (line 200): `kani_gate_08_empty_nodes_valid_accessors_pass` - HARDCODED ✓
  - H10 (line 228): `kani_gate_08_expressions_with_accessor_refs` - HARDCODED ✓
  - H11 (line 286): `kani_gate_08_mixed_accessor_paths` - HARDCODED ✓
  - H12 (line 352): `kani_gate_08_all_node_kinds_no_panic` - uses `kani::any()`, NOT hardcoded ✗
  - H13 (line 363): `kani_gate_08_constants_with_symbols` - HARDCODED ✓
  - H14 (line 414): `kani_gate_08_many_accessors_varied_depths` - HARDCODED ✓
  
**Actual hardcoded count in H9-H14: 5, not 7**

### 3. Vague Language in vld-005
- **Artifact**: verifier-lane-decisions.jsonl, vld-005
- **Issue**: Contains "7 hardcoded WorkflowParts harnesses (9-14 and possibly one more)"
- **Problem**: "possibly one more" is ambiguous and unacceptable in a final proof plan
- **Required fix**: Either identify the exact harness or remove the qualification

### 4. Missing contract-spec.md
- **Artifact**: contract-spec.md (referenced in review checklist but absent)
- **Issue**: Cannot verify contract parity without this artifact
- **Required fix**: Provide contract-spec.md or explain why it's not applicable

### 5. Circular State References
- **Artifact**: verifier-lane-decisions.jsonl and proof-obligations.planned.jsonl
- **Issue**: All lane decisions have `owner_state: 5` and all obligations have `rerun_from: 5`
- **Problem**: State 5 means "execution planned" but obligations haven't been executed yet. The rerun_from field implies prior execution at state 5, which is impossible for a planned obligation.
- **Required fix**: Use `rerun_from: null` for planned obligations, or clarify the state machine semantics

### 6. Ambiguous Harness Target in PO-007
- **Artifact**: proof-obligations.planned.jsonl, PO-007
- **Issue**: Target is "7 hardcoded Capability names in check_capability_* harnesses" but command uses `--harness check_capability_harness`
- **Problem**: 14 harnesses in kani_capability_harnesses.rs share the `check_capability_*` prefix. The command will run only one harness, not all violating ones.
- **Source verification** of hardcoded capability names:
  - strict_admission_invalid_artifact_cases_reject (line 135): hardcoded "network" in accepted_artifact
  - strict_admission_invalid_capability_rejects (line 168): hardcoded "network"
  - strict_admission_valid_artifact_admits (line 225): hardcoded "network"
  - check_capability_grants_exact_match (line 275): hardcoded "action"
  - check_capability_action_match_name_grants (line 288): hardcoded "network"
  - check_capability_action_match_name_denies (line 299): hardcoded "secrets"/"network"
  - check_capability_action_mismatch_name_grants (line 314): hardcoded "network"
  - check_capability_action_mismatch_name_denies (line 329): hardcoded "secrets"/"network"
  - check_capability_hierarchical_rejects_subpath (line 344): hardcoded "network.api"/"network"
  - check_capability_partial_segment_rejected (line 359): hardcoded "network"/"net"
- **Required fix**: Specify exact harness names for each violation or use glob pattern

## GOD RULE Compliance

| Check | Result |
|-------|--------|
| No behavior-affecting waivers | ✓ PASS — waiver-candidates.jsonl is empty |
| Concrete repair actions | ✗ FAIL — PO-005 overcounts by 2 harnesses |
| Vacuous proof elimination | ✗ FAIL — PO-005 incorrectly labels H12 (uses kani::any()) as hardcoded |
| Non-applicability justified | ✓ PASS — all 7 non-kani lanes correctly marked not_applicable |

## Positive Findings

1. **Waiver candidates correctly empty**: No behavior-affecting waivers listed
2. **Lane decisions complete**: All 7 seeds × 8 verifiers = 56 lane decision records
3. **Traceability complete**: All 7 seeds map to source_file/verifier in traceability-matrix.jsonl
4. **Non-vacuity evidence**: Obligations include specific bounds (unwind factors, capacity ranges)
5. **Trusted base correctly scoped**: No production code changes; only kani_*.rs files modified

## Required Repairs

1. **proof-strategy.md**: Fix title to match actual violation count (24-26 depending on categorization)
2. **PO-005**: Correct harness list to 5 hardcoded (H9, H10, H11, H13, H14) + add H5/H7/H8 if needed
3. **vld-005**: Remove "possibly one more" language; state exact harness count
4. **contract-spec.md**: Provide artifact or document why N/A
5. **State semantics**: Clarify owner_state/rerun_from for planned obligations
6. **PO-007 command**: Use `--harness check_capability_` glob or list exact harness names

---

**Report**: STATUS: REJECTED | Lanes reviewed: 1 (kani) | Blockers: 6
