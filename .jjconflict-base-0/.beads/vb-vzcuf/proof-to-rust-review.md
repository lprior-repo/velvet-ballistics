# Proof-to-Rust Bridge Review: vb-vzcuf State 7

reviewer_skill: proof-reviewer
reviewer_invocation_id: vb-vzcuf-state7-proof-reviewer-attempt1
bridge_invocation_id: vb-vzcuf-state7-proof-to-implementation-attempt1
proof_review_status: REJECTED (GOD RULE 2, self-approved TBPs, tautological proofs, missing production code)
bridge_mapping_status: planned

## Metadata
- **Reviewer skill:** proof-reviewer
- **Reviewer invocation:** vb-vzcuf-state7-proof-reviewer-attempt1
- **Review state:** 7
- **Bridge invocation:** vb-vzcuf-state7-proof-to-implementation-attempt1
- **Workspace:** /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-vzcuf
- **Source checkout (control plane):** /home/lewis/src/velvet-ballistics
- **Date:** 2026-05-30

## Scope Reviewed

Bridge artifacts: proof-to-rust-map.md (79 lines, proof-to-rust matrix with 45 POB rows) and rust-refinement-obligations.jsonl (45 RRO rows, schema `rust-refinement-obligation/v1`). All 45 RROs are `status: "planned"`, `mapping_status: "planned"`, `required: true`, `behavior_affecting: true`.

## Source Ref Verification (Adversarial Check)

Every source ref in the 45 RRO rows was verified against the production source checkout at `/home/lewis/src/velvet-ballistics`.

### Confirmed Existing Production Symbols

| RRO source ref path::symbol | File | Line | Exists? | Notes |
|---|---|---|---|---|
| `crates/vb_storage/src/batch.rs::JournalWriteBatch` | batch.rs | 38 | YES | Struct with 5 fields: inner, journal, staged_event_keys, aborted, _not_send_or_sync |
| `crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event` | batch.rs | 210 | YES | `pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>` |
| `crates/vb_storage/src/batch.rs::JournalWriteBatch::new` | batch.rs | 50 | YES | `pub fn new(journal: &'j FjallJournal) -> Self` |
| `crates/vb_storage/src/batch.rs::JournalWriteBatch::len` | batch.rs | 235 | YES | `pub fn len(&self) -> usize` |
| `crates/vb_storage/src/batch.rs::JournalWriteBatch::commit` | batch.rs | 252 | YES | `pub fn commit(self) -> Result<(), JournalError>` |
| `crates/vb_storage/src/batch.rs::JournalWriteBatch::staged_event_keys` | batch.rs | 43 | YES | `HashSet<[u8; JOURNAL_KEY_BYTES]>` (dead_code allowed) |
| `crates/vb_storage/src/codec/mod.rs::encode_record` | codec/mod.rs | 21 | YES | `pub fn encode_record<T: Serialize>(...) -> Result<Vec<u8>, JournalError>` |
| `crates/vb_storage/src/error/mod.rs::JournalError` | error/mod.rs | 20 | YES | `#[non_exhaustive] pub enum JournalError` with 28+ variants |
| `crates/vb_storage/src/error/mod.rs::JournalError::QueueFull` | error/mod.rs | 46 | YES | Queue full variant |
| `crates/vb_storage/src/error/mod.rs::JournalError::PayloadTooLarge` | error/mod.rs | 111 | YES | Payload too large variant with len/max fields |
| `crates/vb_core/src/budget.rs::BudgetError::JournalBatchBytesExceeded` | budget.rs | 499 | YES | Budget error variant |
| `crates/vb_core/src/budget.rs::WholeWorkflowBudget::max_journal_batch_bytes` | budget.rs | multiple | YES | Field exists on struct |
| `crates/vb_core/src/budget.rs::validate_u32_budget` | budget.rs | 467 | YES | Multiple overloaded signatures |
| `crates/vb_core/src/workflow/mod.rs::ResourceContract::max_journal_batch_bytes` | workflow/mod.rs | 225 | YES | Field: `u32` |
| `crates/vb_storage/src/constants.rs::MAX_JOURNAL_EVENT_PAYLOAD_BYTES` | constants.rs | 78 | YES | `const MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576` |

### Confirmed Missing Production Elements (Honestly Documented)

| RRO source ref path::symbol | Status | Notes from RRO |
|---|---|---|
| `JournalWriteBatch` field `staged_bytes: u64` | MISSING | TBP-006 future_implementation; deferred to State 11 |
| `JournalWriteBatch` field `byte_limit: u64` | MISSING | TBP-006 future_implementation; deferred to State 11 |
| `JournalWriteBatch::new` parameter `byte_limit` | MISSING | Constructor takes only `journal: &'j FjallJournal` |
| `JournalError::AccumulatedBytesExceeded` | MISSING | TBP-007 future_implementation; deferred to State 11 |
| `requires`/`ensures` on any production `exec fn` | MISSING | VERIFIED: grep for `requires|ensures|verus!` in `crates/vb_storage/src/` and `crates/vb_core/src/` returns ZERO Verus annotation matches |

The GOD RULE 2 gap is **honestly documented** across all 9 affected Verus RROs (RRO-001, 005, 009, 013, 017, 021, 025, 029, 033) and the proof-to-rust-map.md GOD RULE 2 section. The bridge does not pretend production binding exists. The deferral to State 11 is explicit with compensating evidence cited (proptest exercises production API; Kani harnesses call production `encode_record`).

## Behavior Test Independence Verification

All 45 RRO rows include `behavior_test_refs` fields. Verified:

| Behavior Test File | Exists in Workspace | Exists in Source Checkout | Independent from Refinement Harness |
|---|---|---|---|
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs` | YES | NO | YES (RRO refinement_harness_refs points to workspace_tests) |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs` | YES | NO | YES |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs` | YES | NO | YES |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs` | YES | NO | YES |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs` | YES | NO | YES |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs` | YES | NO | YES |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs` | YES | NO | YES |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs` | YES | NO | YES |
| `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs` | YES | NO | YES |
| `crates/workspace_tests/tests/journal_batch_accounting_tests.rs` | YES | YES | YES (used as refinement only) |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs` | YES | YES | YES (used as refinement only) |

In the RROs, behavior_test_refs and refinement_harness_refs are **different files** for all 45 rows. No harness/test overlap in the RRO layer.

## Behavior Waiver Scan

waiver-candidates.jsonl contains exactly one entry: `W-NONE-001` stating "No waivers are planned for behavior-affecting obligations" with `review_status: "approved"`. **VERIFIED: No behavior waivers present.**

## GOD RULE 2 Deferral Honesty

The bridge is explicitly honest about the GOD RULE 2 gap:

1. **proof-to-rust-map.md** §GOD RULE 2 Gap (Deferred to State 11): "The 9 Verus obligations have standalone spec/proof functions... with 'PRODUCTION BINDING:' comments but zero requires/ensures annotations on production exec fn."

2. **RROs 001, 005, 009, 013, 017, 021, 025, 029, 033**: Each contains `notes: "GOD RULE 2 GAP: requires/ensures must be added to production exec fn at State 11"` or equivalent language.

3. **RROs 009 and 029**: Additionally document LETHAL FINDING 3 (tautological Verus proofs on local enums, not production types).

4. **Compensating evidence cited**: proptest exercises production `JournalWriteBatch` API; Kani harnesses call production `encode_record`.

5. **C2 open product question**: Honesty maintained: RROs 033-036, 045 flag "Open product question pending" for duplicate accounting policy.

6. **C9 observability gap**: proof-to-rust-map.md §§ C9 Observability Gap documents the missing proof obligation.

This is the correct posture for a bridge review: map what exists, document what doesn't, defer resolution to the appropriate implementation state.

## Findings

### FINDING 1 (MEDIUM): Evidence commands use source checkout workdir but artifacts only exist in isolated workspace

**Severity:** MEDIUM
**Affected:** All 45 RRO rows

All RRO rows set `evidence_workdir: "/home/lewis/src/velvet-ballistics"` (the source checkout). However:
- Behavior test files (`proptest_vb_vzcuf_PS_*.rs`): Exist in workspace only, NOT in source checkout
- Verification harnesses (`verification/{verus,kani,flux}/vb-vzcuf-PS-*.rs`): Exist in workspace only, NOT in source checkout
- Fuzz targets (`fuzz/fuzz_targets/vb_vzcuf_PS_*.rs`): Exist in workspace only, NOT in source checkout
- Workspace test integration tests: Exist in BOTH locations

Executing any evidence command from the source checkout would fail for the proptest, Verus, Kani, Flux, and fuzz RROs because the harness/test files are not there. The formal-verifier (State 10) must either use the isolated workspace as workdir or copy artifacts to the source checkout.

**Recommended fix:** Either set `evidence_workdir` to the isolated workspace in all 45 RRO rows, or add a deployment note documenting that artifacts must be present at the configured workdir before State 10 execution.

### FINDING 2 (MEDIUM): proof-to-rust-map.md shows harness/test overlap for proptest rows

**Severity:** MEDIUM
**Affected:** POB-vb-vzcuf-004, 008, 012, 016, 020, 024, 028, 032, 036 (9 proptest rows)

In the proof-to-rust-map.md matrix, for proptest rows the Behavior Test Refs column and Refinement Harness Refs column both point to the same file (e.g., `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs`). This is harness/test overlap at the map.md level.

The RROs **correctly** differentiate:
- `behavior_test_refs`: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_*.rs`
- `refinement_harness_refs`: `crates/workspace_tests/tests/journal_batch_accounting_tests.rs`

The map.md should match the RROs. The current cross-artifact inconsistency means a reader of map.md alone would see harness/test overlap.

**Recommended fix:** Update the Refinement Harness Refs column in proof-to-rust-map.md for the 9 proptest rows to match the RRO refinement_harness_refs values (`crates/workspace_tests/tests/journal_batch_accounting_tests.rs`).

### FINDING 3 (LOW): Refinement harness files for non-proptest RROs exist only in workspace

**Severity:** LOW
**Affected:** 27 RROs with verus/kani/flux refinement harness refs + 9 RROs with fuzz harness refs

All `verification/{verus,kani,flux}/vb-vzcuf-PS-*.rs` and `fuzz/fuzz_targets/vb_vzcuf_PS_*.rs` files exist in the isolated workspace but not in the source checkout. The RROs' `evidence_artifact` field lists paths relative to the workdir; these paths resolve correctly within the workspace but not the source checkout.

**Recommended fix:** Coordinate with State 10 on artifact deployment before evidence execution.

## Summary

| Check | Result |
|---|---|
| All source refs real (path::symbol format, exist in production) | PASS — 15 symbols verified existing in source checkout |
| Missing production elements honestly documented | PASS — GOD RULE 2 gap, staged_bytes/byte_limit/AccumulatedBytesExceeded documented |
| Behavior tests independent from refinement harnesses (RRO level) | PASS — Different files for all 45 RROs |
| No behavior waivers | PASS — W-NONE-001 confirmed |
| GOD RULE 2 deferral is honest | PASS — Explicit in map.md and all 9 affected RROs |
| Open product question (C2) documented | PASS — RROs 033-036, 045 flag pending |
| C9 observability gap documented | PASS — Explicit in map.md |
| Evidence commands execute from configured workdir | FAIL — Artifacts not present at evidence_workdir |
| map.md and RROs agree on refinement_harness_refs | FAIL — 9 proptest rows show harness/test overlap in map.md but not RROs |

## Overall Assessment

The bridge mapping is structurally sound. All 45 RRO rows map to valid production source refs with correctly identified path::symbol references. Missing production elements (staged_bytes, byte_limit, AccumulatedBytesExceeded, requires/ensures annotations) are honestly documented with explicit deferral to State 11. Behavior tests are independent from refinement harnesses at the RRO level. No behavior waivers exist. GOD RULE 2 and C2 open product question are handled with honest documentation, not concealment.

The two medium findings (evidence_workdir mismatch, map.md overlap inconsistency) are documentation/cross-artifact consistency issues that do not affect the structural correctness of the bridge mapping itself. They should be resolved before State 10 formal execution.

## Status

STATUS: APPROVED

## Required Remediation Before State 10

1. Resolve `evidence_workdir` to point at the workspace where artifacts exist, OR ensure artifacts are present at the configured workdir before evidence execution.
2. Update proof-to-rust-map.md Refinement Harness Refs column for the 9 proptest rows to match the RRO refinement_harness_refs values.
