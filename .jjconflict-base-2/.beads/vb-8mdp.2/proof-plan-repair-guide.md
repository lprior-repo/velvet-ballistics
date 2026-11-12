# Proof Plan Repair Guide: vb-8mdp.2 (Attempt 3)

## Review History
- **Attempt 1**: REJECTED — 5 critical findings
- **Attempt 2**: REJECTED — All 5 findings still open (this review)

## All 5 Findings Remain Open

This guide documents exactly what must change for the next review cycle.

---

## Repair 1: DELETE PO-006 from proof-obligations.planned.jsonl

**File**: `proof-obligations.planned.jsonl`

**Action**: Remove the entire line containing `"id":"PO-006"`.

**Current content to remove**:
```json
{"id":"PO-006","requirement_id":"REQ-STORAGE-011","contract_clause":"C-BUDGET-003","risk":"budget_overflow","verifier":"rust-type-system","artifact":"crates/vb_storage/src/codec/header.rs","target":"crates/vb_storage/src/codec/header.rs:decode_record_header:26","command":"N/A - code review","expected_evidence":"Function signature is decode_record_header(header: &[u8], ...) -> &[u8] borrows, cannot create Vec","assumptions":["Rust borrow checker prevents allocation inside function taking only &[u8]"],"bounds":"N/A","required":true,"mode":"code-review","owner_state":4,"rerun_from":4,"status":"proven","waiver":null}
```

**Reason**: Code-review is not an acceptable proof mode for safety-critical budget-overflow obligations. The "no Vec" property is already proven by PO-001 (Kani) and PO-003 (Verus).

---

## Repair 2: Add reviewer_invocation_id to verifier-lane-review.jsonl

**File**: `verifier-lane-review.jsonl`

**Action**: Add `"reviewer_invocation_id": "proof-plan-reviewer/vb-8mdp.2/20260525-re2"` to every of the 26 rows.

**Example — before**:
```json
{"verifier":"kani","proof_seed_id":"vb-8mdp-2-ps-001","review_note":"...","reviewer":"proof-plan-reviewer","status":"needs_attention"}
```

**Example — after**:
```json
{"verifier":"kani","proof_seed_id":"vb-8mdp-2-ps-001","review_note":"...","reviewer":"proof-plan-reviewer","reviewer_invocation_id":"proof-plan-reviewer/vb-8mdp.2/20260525-re2","status":"needs_attention"}
```

Do this for all 26 rows. The reviewer_invocation_id proves independent invocation from the planner.

---

## Repair 3: Fix 11 proof-seeds.jsonl evidence_command fields

**File**: `proof-seeds.jsonl`

**Action**: Update `evidence_command` field in these 11 proof seeds to match the harness names in the obligations:

| Proof Seed | Current | Correct |
|------------|---------|---------|
| ps-001 | `cargo kani --package vb_storage --harness kani_codec` | `cargo kani --package vb_storage --harness kani_budget_payload_too_large` |
| ps-002 | `cargo kani --package vb_storage --harness kani_codec` | `cargo kani --package vb_storage --harness kani_header_total_function` |
| ps-003 | `cargo kani --package vb_storage --harness kani_record_payload_len` | `cargo kani --package vb_storage --harness kani_payload_slice_bounds` |
| ps-004 | `cargo kani --package vb_storage --harness kani_record_payload_len` | `cargo kani --package vb_storage --harness kani_payload_overflow_check` |
| ps-005 | `cargo kani --package vb_storage --harness kani_record_magic` | `cargo kani --package vb_storage --harness kani_magic_order` |
| ps-006 | `cargo kani --package vb_storage --harness kani_record_kind` | `cargo kani --package vb_storage --harness kani_unknown_kind` |
| ps-008 | `cargo kani --package vb_storage --harness kani_codec` | `cargo kani --package vb_storage --harness kani_header_length_mismatch` |
| ps-009 | `cargo kani --package vb_storage --harness kani_record_schema` | `cargo kani --package vb_storage --harness kani_schema_versions` |
| ps-010 | `cargo kani --package vb_storage --harness kani_record_crc` | `cargo kani --package vb_storage --harness kani_crc_mismatch` |
| ps-013 | `cargo kani --package vb_storage --harness kani_codec` | `cargo kani --package vb_storage --harness kani_journal_event_semantic` |
| ps-015 | `cargo kani --package vb_storage --harness kani_codec` | `cargo kani --package vb_storage --harness kani_blob_budget` |

---

## Repair 4: Add new-artifact note to TLA+ obligations

**File**: `proof-obligations.planned.jsonl`

**Action**: Add `"note": "NEW ARTIFACT — must be created by proof-writer before running tlc"` to PO-019 and PO-020.

**PO-019 — after**:
```json
{"id":"PO-019","note": "NEW ARTIFACT — must be created by proof-writer before running tlc", ...}
```

**PO-020 — after**:
```json
{"id":"PO-020","note": "NEW ARTIFACT — must be created by proof-writer before running tlc", ...}
```

---

## Repair 5: Fix PO-021 Kani command syntax

**File**: `proof-obligations.planned.jsonl`

**Action**: Change PO-021 command from:
```
cargo kani --fuzz decode_record --harness kani_decode_record_arbitrary
```
to either:
```
cargo kani --fuzz decode_record
```
OR:
```
cargo kani --harness kani_decode_record_arbitrary --package vb_storage
```

**Reason**: `--fuzz` and `--harness` cannot be combined. `cargo kani --fuzz <target>` runs all harnesses in a fuzz target. `cargo kani --harness <name>` runs a specific harness in the crate. These are mutually exclusive.

---

## Summary Checklist

- [ ] PO-006 DELETED from proof-obligations.planned.jsonl
- [ ] reviewer_invocation_id added to all 26 rows in verifier-lane-review.jsonl
- [ ] 11 evidence_command fields fixed in proof-seeds.jsonl
- [ ] NEW ARTIFACT note added to PO-019 and PO-020
- [ ] PO-021 command syntax fixed
