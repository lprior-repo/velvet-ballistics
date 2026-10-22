# Semantic Audit: vb-fyktj Own Commit 0f71f398

**Historical own commit:** `0f71f398cf66a3c8acb881826683aa36f6d7c850` (origin/bead/vb-3y9q5)
**Current frozen source:** `0dd55b94cbdd05c21951e6e239313d393816a99d`
**Files audited:** 6
**Own unified hunks audited:** 23
**Date:** 2026-10-22

## Scope

This audit compares the exact 23 unified hunks introduced in own commit 0f71f398 against the current frozen source at 0dd55b94. The comparison is hunk-by-hunk, checking actual implementation bodies and assertion values — not method name presence alone.

## Summary

| Status | Count |
|--------|-------|
| Exact present | 22 |
| Semantically present | 1 |
| Superseded with replacement | 0 |
| Missing useful | 0 |
| Obsolete | 0 |
| Unresolved | 0 |

**Conclusion:** All 23 own unified hunks from commit 0f71f398 are present in the current frozen source. One test hunk (tests.rs hunk 3) is semantically present with identical assertion values but differs in line wrapping (4 non-empty added lines not byte-identical to historical). The production file frame_pool.rs is whole-file byte-identical to the historical commit. No hunks were lost, superseded, or drifted.

## Per-File Results

### crates/vb_runtime/src/frame_pool.rs (1 unified hunk)

| Hunk | Change | Status | Evidence |
|------|--------|--------|----------|
| FP-H1 | Pre-allocate all frames at construction; take() no fresh allocation fallback; release() dimension + capacity checks | Exact present | Lines 29-87: all three method implementations match historical; whole file byte-identical to historical commit |

### crates/vb_runtime/src/frame_pool/tests.rs (16 unified hunks)

| Hunk | Change | Status | Evidence |
|------|--------|--------|----------|
| T-H1 | Rename take_allocates_when_empty -> take_returns_preallocated_frame | Exact present | Line 44: exact name and body match |
| T-H2 | available() == 4 after release (was 1) | Exact present | Line 60: assert_eq!(pool.available(), 4) |
| T-H3 | Multiple test changes including 3 new tests; 4 non-exact lines (line wrapping only) | Semantically present | Lines 67-162: all 7 logical changes present with identical assertion values; 4 lines differ only in line wrapping |
| T-H4 | available() == 8 (was 0) | Exact present | Line 171: assert_eq!(pool.available(), 8) |
| T-H5 | available() == 4 after release (was 1) | Exact present | Line 196: assert_eq!(pool.available(), 4) |
| T-H6 | Comment change only | Exact present | Line 209: exact comment |
| T-H7 | available() == 4 (was 1) | Exact present | Line 231: assert_eq!(pool.available(), 4) |
| T-H8 | available() == 4 then 3 (was 2 then 1) | Exact present | Lines 293, 297: exact assertions |
| T-H9 | Rename and available() == 4 (was 0) | Exact present | Line 330: exact name; line 333: assert_eq!(pool.available(), 4) |
| T-H10 | Rename and available() == 4 (was 0) | Exact present | Line 475: exact name; line 480: assert_eq!(pool.available(), 4) |
| T-H11 | Rename and f3 fails with AllocationFailed; wrong-dimension test changes | Exact present | Lines 497-543: exact test bodies |
| T-H12 | Rename and construction itself fails | Exact present | Lines 552-569: exact test body |
| T-H13 | Only 2 frames exist (was 5) | Exact present | Lines 633-648: exact test body |
| T-H14 | available() == 4096 (was 0) | Exact present | Line 671: assert_eq!(pool.available(), 4096) |
| T-H15 | available() == 4 (was 0) | Exact present | Lines 697-714: exact test body |
| T-H16 | Rename and second take fails | Exact present | Lines 724-736: exact test body |

### crates/vb_runtime/src/shard/tests/chunk_003.rs (3 unified hunks)

| Hunk | Change | Status | Evidence |
|------|--------|--------|----------|
| C3-H1 | available() == Some(4) (was Some(1)) | Exact present | Line 113: assert_eq!(available, Some(4)) |
| C3-H2 | First assertion: available() == Some(3) (was Some(0)) | Exact present | Line 134: Some(3) |
| C3-H3 | Second assertion: available() == Some(4) (was Some(1)) | Exact present | Line 145: Some(4) |

### crates/vb_runtime/src/shard/tests/chunk_010.rs (1 unified hunk)

| Hunk | Change | Status | Evidence |
|------|--------|--------|----------|
| C10-H1 | available() == Some(4) (was Some(1)) | Exact present | Line 38: Some(4) |

### crates/vb_runtime/src/shard/tests/chunk_017.rs (1 unified hunk)

| Hunk | Change | Status | Evidence |
|------|--------|--------|----------|
| C17-H1 | Rename and f3 fails with AllocationFailed | Exact present | Lines 208-222: exact test body |

### crates/vb_runtime/src/shard/tests/chunk_024.rs (1 unified hunk)

| Hunk | Change | Status | Evidence |
|------|--------|--------|----------|
| C24-H1 | First assertion: Some(3) (was Some(0)); second: Some(4) (was Some(1)) | Exact present | Lines 192-194: Some(3); lines 199-201: Some(4) |

## Notes

- Frame pool production implementation (frame_pool.rs) is whole-file exact equal to historical commit.
- Test files are not whole-file exact due to 4 non-empty added lines in tests.rs hunk 3 that differ only in line wrapping/formatting; assertion values and behavior are identical.
- The existing frame pool no-allocation cycle test (frame_pool_no_runtime_allocation_after_construction) does not measure actual allocator state; it only verifies available() remains at capacity. This is an existing test weakness, not a missing hunk restoration need.
- Several historical tests used early-return on setup failure (e.g., Err(_) => return) rather than panic; this pattern remains and is an existing test quality limitation, not a missing hunk.

This audit is a provisional diagnostic, pending root actual-scope and evidence cross-check. It does not constitute main acceptance.