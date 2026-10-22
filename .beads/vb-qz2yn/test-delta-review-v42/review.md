# Test Delta Review: vb-qz2yn

**Bead:** vb-qz2yn
**State:** 2
**Reviewer:** local5090
**Date:** 2026-10-22
**Source commit:** 4b7248a43322d3c58b097b67ef05bbb71329b21a
**Scope:** One existing test in one test file (11th file, not in v27 approval)

**STATUS: APPROVED**

## Delta Under Review

File: `crates/vb_storage/tests/accepted_artifact_red_phase.rs` (line 244-249)

Old:
```rust
fn accepted_artifact_encoder_records_zero_accepted_at_seq() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    // Current implementation records accepted artifacts without journal sequence tracking.
    assert_eq!(artifact.accepted_at_seq.get(), 0);
    Ok(())
}
```

New:
```rust
fn accepted_artifact_submission_records_first_accepted_at_seq() -> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    // Fresh journal submissions start at the first issued artifact sequence.
    assert_eq!(artifact.accepted_at_seq.get(), 1);
    Ok(())
}
```

Change: Three lines (name, comment, assert literal). One test in this file only. All other fixtures, test bodies, and the other 29 tests in this file are unchanged.

## Contract Compliance

Contract requirement: "First issued sequence is 1 (checked_add from 0)"

The new test verifies this exact contract behavior: a fresh journal's first submission returns `accepted_at_seq == 1`. The old test was a known-GAP-007 placeholder asserting seq=0 ("without journal sequence tracking").

## Verification of Delta Properties

- [x] One assertion literal changed: 0 → 1
- [x] Test fixture (`submit_minimal`) unchanged: temp_journal + minimal workflow
- [x] Test body unchanged: one submit, one assertion
- [x] No fixture weakening
- [x] No assertion removed, skipped, or ignored
- [x] No production code changed
- [x] All other 29 tests in file preserved
- [x] Test name updated to reflect actual behavior (submission path, not encoder)
- [x] Comment updated to explain real behavior, not legacy placeholder

## Relationship to v27 Approval

v27 approved 13 new tests across 4 new modules and 6 updated existing test files. This file (`accepted_artifact_red_phase.rs`) is a pre-existing test file that was not modified by v27. The old seq=0 assertion here was a documented legacy placeholder (GAP-007) that remained because it was outside v27's scope. This delta closes that legacy placeholder now that the counter implementation is complete.

## Execution Context

Per prompt: V36 full run failed 1 test with old assert=0 (this exact test), 9388 passed. The focused execution of this specific test with seq=1 is expected to pass but is not asserted in this review — this is a test source review only.

## Findings

None. The delta is mechanically correct: one assert literal updated from legacy placeholder value (0) to actual contract-specified value (1) for a fresh-journal first submission. Test is stronger now (verifies real behavior) and matches contract.
