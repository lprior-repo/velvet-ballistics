# Semantic Audit: vb-fyktj Own Commit 0f71f398

**Historical own commit:** `0f71f398cf66a3c8acb881826683aa36f6d7c850` (origin/bead/vb-3y9q5)
**Current frozen source:** `0dd55b94cbdd05c21951e6e239313d393816a99d`
**Files audited:** 6
**Own hunks audited:** 30
**Date:** 2026-10-22

## Scope

This audit compares the exact hunks introduced in my own commit 0f71f398 against the current frozen source at 0dd55b94. The comparison is hunk-by-hunk, checking actual implementation bodies and assertion values — not method name presence alone.

The prior audit (8 commits ago) falsely marked all hunks as "exact present" based on method names only. This audit verifies actual code content and behavior.

## Summary

| Status | Count |
|--------|-------|
| Exact present | 30 |
| Semantically present | 0 |
| Superseded with replacement | 0 |
| Missing useful | 0 |
| Obsolete | 0 |
| Unresolved | 0 |

**Conclusion:** All 30 own hunks from commit 0f71f398 are exactly present in the current frozen source. No hunks were lost, superseded, or drifted.

## Per-File Results

### crates/vb_runtime/src/frame_pool.rs (3 hunks)

| Hunk | Change | Status | Evidence |
|------|--------|--------|----------|
| H1: `new()` | Pre-allocate all frames at construction | Exact present | Lines 30-49: loop creates `capacity` frames via `RunFrame::new`, stores in `frames` Vec |
| H2: `take()` | No fresh allocation fallback | Exact present | Lines 57-65: only `self.frames.pop()`, returns `AllocationFailed` if empty |
| H3: `release()` | Dimension + capacity checks | Exact present | Lines 70-87: checks `step_count`/`slot_count` match, checks `len() >= capacity`, then pushes |

### crates/vb_runtime/src/frame_pool/tests.rs (21 hunks)

| Test | Assertion | Status | Evidence |
|------|-----------|--------|----------|
| `take_returns_preallocated_frame` | renamed from `take_allocates_when_empty` | Exact present | Line 44: exact name and body match |
| `release_and_reuse_frame` | `available() == 4` after release | Exact present | Line 62: `assert_eq!(pool.available(), 4)` |
| `release_drops_when_at_capacity` | second `take()` fails | Exact present | Lines 74-78: `AllocationFailed` match on frame2 |
| `is_empty_returns_false_for_new_pool` | `is_empty() == false` | Exact present | Line 92: exact assertion with message |
| `is_empty_returns_true_after_taking_all` | takes 4, then `is_empty() == true` | Exact present | Lines 99-113: loop takes 4 frames |
| `capacity_returns_configured_value` | `available() == 42` | Exact present | Line 117: `assert_eq!(pool.available(), 42, ...)` |
| `frame_pool_new_preallocates_capacity_frames` | new test, `available() == 8` | Exact present | Lines 120-128: exact test body |
| `frame_pool_no_runtime_allocation_after_construction` | new test, 100 cycles | Exact present | Lines 131-145: exact test body |
| `frame_pool_exhausted_take_returns_error` | new test, 3rd take fails | Exact present | Lines 148-162: exact test body |
| `frame_pool_new_creates_pool_with_capacity` | `available() == 8` | Exact present | Line 171: exact assertion |
| `frame_pool_release_returns_frame_to_pool` | `available() == 4` | Exact present | Line 186: exact assertion |
| `frame_pool_is_empty_when_all_acquired` | comment change only | Exact present | Line 197: exact comment |
| `frame_pool_is_not_empty_after_release` | `available() == 4` | Exact present | Line 221: exact assertion |
| `frame_pool_multiple_release_and_take_cycle` | `available() == 4` then 3 | Exact present | Lines 283-285: exact assertions |
| `frame_pool_take_returns_preallocated_frame_when_no_recycled` | renamed | Exact present | Line 330: exact name and body |
| `frame_pool_available_starts_at_capacity` | `available() == 4` | Exact present | Line 469: exact assertion |
| `frame_pool_exhausted_take_returns_error_no_fresh_alloc` | renamed, f3 fails | Exact present | Lines 492-508: exact test body |
| `frame_pool_release_wrong_dimension_frame_is_silently_dropped` | `available() == 4` | Exact present | Lines 514-531: exact assertions |
| `frame_pool_zero_step_count_rejects_construction` | renamed, construction fails | Exact present | Lines 545-562: exact test body |
| `frame_pool_release_after_release_at_capacity_drops_all_extras` | 2 frames only | Exact present | Lines 628-640: exact test body |
| `frame_pool_large_capacity_at_boundary_succeeds` | `available() == 4096` | Exact present | Line 664: exact assertion |
| `frame_pool_concurrent_dimension_pools_do_not_interfere` | `available() == 4` | Exact present | Lines 695-708: exact assertions |
| `frame_pool_take_returns_error_when_exhausted` | renamed, second take fails | Exact present | Lines 717-729: exact test body |

### crates/vb_runtime/src/shard/tests/chunk_003.rs (2 hunks)

| Test | Assertion | Status | Evidence |
|------|-----------|--------|----------|
| `finished_run_releases_frame_to_dimension_pool` | `available() == Some(4)` | Exact present | Line 111: exact assertion with comment |
| `cancelled_run_releases_frame_to_dimension_pool` | `available() == Some(3)` then `Some(4)` | Exact present | Lines 134, 145: exact assertions with comments |

### crates/vb_runtime/src/shard/tests/chunk_010.rs (1 hunk)

| Test | Assertion | Status | Evidence |
|------|-----------|--------|----------|
| `shard_submit_run_reuses_frame_from_pool_after_prior_finish` | `available() == Some(4)` | Exact present | Lines 38-40: exact assertion with comment |

### crates/vb_runtime/src/shard/tests/chunk_017.rs (1 hunk)

| Test | Assertion | Status | Evidence |
|------|-----------|--------|----------|
| `bh_shd_07_frame_pool_bounded_no_fresh_alloc` | renamed, f3 `AllocationFailed` | Exact present | Lines 201-214: exact test body |

### crates/vb_runtime/src/shard/tests/chunk_024.rs (1 hunk)

| Test | Assertion | Status | Evidence |
|------|-----------|--------|----------|
| `vb1u88_cancel_removes_run_and_releases_frame` | `available() == Some(3)` then `Some(4)` | Exact present | Lines 179, 190: exact assertions with comments |

## Methodology

1. Read the manifest to identify files and packet parts.
2. Read the full historical patch (`historical-own.patch`) to identify all own hunks.
3. For each hunk, located the corresponding code in the current frozen source using `read` with raw selector to view full implementation bodies.
4. Compared actual assertion values, function bodies, and comments — not just method names.
5. Recorded exact line anchors and matching evidence for each hunk.

This audit is a provisional diagnostic, pending root actual-scope and evidence cross-check. It does not constitute main acceptance.
