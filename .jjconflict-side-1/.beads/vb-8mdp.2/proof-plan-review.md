# Proof Plan Review: vb-8mdp.2 Budget-Before-Decode (RE-REVIEW)

## Review Information
- **Reviewer Skill**: proof-plan-reviewer
- **Reviewer Invocation**: proof-plan-reviewer/vb-8mdp.2/20260525-re2
- **Review State**: REJECTED (Attempt 2)
- **Reviewed Artifacts**: proof-obligations.planned.jsonl, verifier-lane-review.jsonl, proof-seeds.jsonl, proof-coverage-matrix.md
- **Prior Review**: Attempt 1 (REJECTED, 5 critical findings)
- **Prior Review Invocation**: proof-plan-reviewer/vb-8mdp.2/20260525

## Summary

This is a re-review of the repaired proof plan after Attempt 1 rejection. The repairs were **not applied**. All 5 critical findings from Attempt 1 remain unfixed. The plan must be rejected again.

---

## Critical Findings (All 5 Still Open)

### Finding 1: PO-006 still "proven" via code-review — NOT REMOVED

**Code**: `E_LANE_DECISION_WEAK`

**Location**: `proof-obligations.planned.jsonl` PO-006 (line 6)

**Problem**: PO-006 was NOT removed as required by Attempt 1. It was reclassified from bare `code-review` to `rust-type-system` with `verifier: "rust-type-system"`, but it still has:
- `"status": "proven"`
- `"mode": "code-review"`
- `"command": "N/A - code review"`

This is still a **type-system observation**, not a formal proof. The Rust borrow checker guarantees `decode_record_header(&[u8])` cannot modify the slice, but it does NOT prove no Vec is allocated in the call chain before line 48. The actual proof depends on PO-001 (Kani) and PO-003 (Verus).

**Verification Lane Policy**: Code-review is not an acceptable proof mode for safety-critical budget-overflow obligations. PO-006 must be **removed** from `proof-obligations.planned.jsonl`, not reclassified.

**Evidence**:
```json
{"id":"PO-006","requirement_id":"REQ-STORAGE-011","contract_clause":"C-BUDGET-003","risk":"budget_overflow","verifier":"rust-type-system","target":"crates/vb_storage/src/codec/header.rs:decode_record_header:26","command":"N/A - code review","expected_evidence":"Function signature is decode_record_header(header: &[u8], ...) -> &[u8] borrows, cannot create Vec","status":"proven","mode":"code-review",...}
```

**Required Repair**: Delete PO-006 from `proof-obligations.planned.jsonl`. The "no Vec in decode_record_header" property is already covered by PO-001 (Kani) and PO-003 (Verus). Do not mark a code-review observation as "proven".

---

### Finding 2: Missing reviewer_invocation_id — NOT FIXED

**Code**: `E_REVIEW_PROVENANCE_MISSING`

**Location**: `verifier-lane-review.jsonl` (all 26 rows)

**Problem**: Every row still has only `"reviewer": "proof-plan-reviewer"` with no `reviewer_invocation_id` field. Attempt 1 required adding `"reviewer_invocation_id": "proof-plan-reviewer/vb-8mdp.2/20260525-re2"` to each row to demonstrate independent invocation.

**Evidence** (first 3 rows as representative sample):
```json
{"verifier":"kani","proof_seed_id":"vb-8mdp-2-ps-001","reviewer":"proof-plan-reviewer","status":"needs_attention"}
{"verifier":"kani","proof_seed_id":"vb-8mdp-2-ps-002","reviewer":"proof-plan-reviewer","status":"needs_attention"}
{"verifier":"kani","proof_seed_id":"vb-8mdp-2-ps-003","reviewer":"proof-plan-reviewer","status":"needs_attention"}
```

No `reviewer_invocation_id` field present in any of the 26 rows.

**Required Repair**: Add `"reviewer_invocation_id": "proof-plan-reviewer/vb-8mdp.2/20260525-re2"` to every row in `verifier-lane-review.jsonl`.

---

### Finding 3: Proof-seeds.jsonl evidence commands still mismatched — NOT FIXED

**Code**: `E_COMMAND_EVIDENCE_MISSING`

**Location**: `proof-seeds.jsonl` ps-001, ps-002, ps-003, ps-004, ps-005, ps-006, ps-008, ps-009, ps-010, ps-013, ps-015

**Problem**: Attempt 1 required updating `evidence_command` fields to match the new artifact names in the obligations. The evidence commands in `proof-seeds.jsonl` still reference OLD harness names that do not match the obligations.

**Specific mismatches** (11 proof seeds affected):

| Proof Seed | Current evidence_command | Should Be |
|------------|------------------------|-----------|
| ps-001 | `kani_codec` | `kani_budget_payload_too_large` |
| ps-002 | `kani_codec` | `kani_header_total_function` |
| ps-003 | `kani_record_payload_len` | `kani_payload_slice_bounds` |
| ps-004 | `kani_record_payload_len` | `kani_payload_overflow_check` |
| ps-005 | `kani_record_magic` | `kani_magic_order` |
| ps-006 | `kani_record_kind` | `kani_unknown_kind` |
| ps-008 | `kani_codec` | `kani_header_length_mismatch` |
| ps-009 | `kani_record_schema` | `kani_schema_versions` |
| ps-010 | `kani_record_crc` | `kani_crc_mismatch` |
| ps-013 | `kani_codec` | `kani_journal_event_semantic` |
| ps-015 | `kani_codec` | `kani_blob_budget` |

**Evidence** (sample from ps-001 and ps-005):
```
ps-001: "evidence_command":"cargo kani --package vb_storage --harness kani_codec"
  (obligation PO-001 uses: cargo kani --package vb_storage --harness kani_budget_payload_too_large)

ps-005: "evidence_command":"cargo kani --package vb_storage --harness kani_record_magic"
  (obligation PO-008 uses: cargo kani --package vb_storage --harness kani_magic_order)
```

**Required Repair**: Update `evidence_command` in every mismatched proof seed to match the exact harness name in the corresponding obligation.

---

### Finding 4: TLA+ artifact creation not noted in obligations — NOT FIXED

**Code**: `E_TLA_NO_RUST_BRIDGE`

**Location**: `proof-obligations.planned.jsonl` PO-019, PO-020

**Problem**: Attempt 1 required noting that `specs/constants.tla` and `specs/budget_before_decode.tla` are new artifacts to be created before `tlc` can run. The obligations still do not contain this note.

**Evidence**:
```
PO-019: "artifact":"specs/constants.tla" — no notation that this is a NEW artifact
PO-020: "artifact":"specs/budget_before_decode.tla" — no notation that this is a NEW artifact
```

These files do not exist at `specs/tla/` in the source checkout.

**Required Repair**: Add `"note": "NEW ARTIFACT — must be created by proof-writer before running tlc"` to PO-019 and PO-020 in `proof-obligations.planned.jsonl`.

---

### Finding 5: cargo kani --fuzz syntax still invalid — NOT FIXED

**Code**: `E_COMMAND_EVIDENCE_MISSING`

**Location**: `proof-obligations.planned.jsonl` PO-021

**Problem**: Attempt 1 flagged that `cargo kani --fuzz decode_record --harness kani_decode_record_arbitrary` uses `--fuzz` and `--harness` in combination, which is not valid. The command still uses this invalid syntax.

**Evidence**:
```json
{"id":"PO-021","command":"cargo kani --fuzz decode_record --harness kani_decode_record_arbitrary",...}
```

Standard `cargo kani` usage is either:
- `cargo kani --harness <harness>` (run a specific harness in the crate)
- `cargo kani --fuzz <target>` (run all harnesses in a fuzz target)

These cannot be combined with `--harness` targeting a harness inside a fuzz target.

**Required Repair**: Change command to either `cargo kani --fuzz decode_record` (runs all harnesses in the fuzz target) OR `cargo kani --harness kani_decode_record_arbitrary --package vb_storage` (runs a specific harness directly).

---

## Verdict

**STATUS: REJECTED**

### Summary of Attempt 2
- All 5 critical findings from Attempt 1 remain unfixed
- No PO-006 removal occurred (only reclassification)
- No reviewer_invocation_id added
- 11 proof-seeds.jsonl evidence commands still mismatched
- TLA+ obligations still lack new-artifact notation
- Kani --fuzz syntax still invalid

### Required Repairs (unchanged from Attempt 1):

1. **DELETE PO-006** from `proof-obligations.planned.jsonl` — not reclassify, DELETE
2. **Add `reviewer_invocation_id`** to every row in `verifier-lane-review.jsonl`
3. **Fix 11 proof-seeds.jsonl** evidence_command fields to match obligation harness names
4. **Add new-artifact note** to PO-019 and PO-020 TLA+ obligations
5. **Fix PO-021** Kani command syntax — remove `--harness` from `--fuzz` invocation

### Valid Aspects (unchanged):
- Non-applicable lane decisions (Flux, Miri, Loom) remain properly justified
- vb-3t44 duplicate avoidance remains correctly documented
- Lane decision coverage (26 rows, 20 proof seeds) remains complete
- Trusted-base plan structure remains sound

### Evidence of Review:

This reviewer (proof-plan-reviewer/vb-8mdp.2/20260525-re2) independently reviewed:
- `/home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-2/proof-obligations.planned.jsonl` (24 lines, PO-001 through PO-024)
- `/home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-2/verifier-lane-review.jsonl` (26 rows)
- `/home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-2/proof-seeds.jsonl` (20 proof seeds)
- `/home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-2/proof-coverage-matrix.md`

All 5 prior findings remain present and blocking.
