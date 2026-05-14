bead_id: vb-p5so
bead_title: "runtime: Forcefully clear pending suspended timers on drain_for_shutdown"
phase: 13
updated_at: 2026-05-09T00:00:00Z

# Architectural Drift Review

## File Line Counts

| File | Before | After | Threshold | Status |
|---|---|---|---|---|
| `crates/vb_runtime/src/shard/impl_.rs` | 788 | 789 | 300 | Already over (pre-existing) |
| `crates/vb_runtime/src/shard/tests.rs` | 6789 | 6974 | 300 | Already over (pre-existing) |
| `crates/vb_runtime/src/shard/lifecycle.rs` | 2040 | 2040 | 300 | Already over (pre-existing) |

**Note**: No file crossed the 300-line threshold due to this change. All target files were already above the limit before work began. Refactoring them is out of scope for this bug-fix bead.

## DDD Assessment

### Primitive Obsession
- No new primitives introduced. ✓
- `pending_timers.clear()` operates on existing `IndexMap<RunId, PendingTimer>` — no primitive abuse. ✓

### Workflows / State Transitions
- No new workflows introduced. ✓
- The change is within the existing shutdown workflow. ✓

### Parse, Don't Validate
- No parsing boundaries touched. ✓

### Functional Core / Imperative Shell
- `drain_for_shutdown` is correctly in the imperative shell (it performs I/O side effects). ✓
- The fix adds one more side effect to the same shell method. ✓

## Refactor Recommendation
None. The change is a minimal 1-line bug fix with zero architectural impact.

STATUS: APPROVED
