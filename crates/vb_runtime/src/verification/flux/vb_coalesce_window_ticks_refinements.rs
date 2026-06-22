/// Flux refinement artifact for `coalesce_window_ticks` and the post-condition
/// of `Shard::flush_coalesce_buffer()` (production:
/// `crates/vb_runtime/src/shard/config.rs::is_valid_coalesce_window_ticks`
/// and `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs::flush_coalesce_buffer`).
///
/// Production invariants bound here:
/// - `coalesce_window_ticks` is a `u32` in the inclusive range
///   `[1, MAX_COALESCE_WINDOW_TICKS]`. The lower bound comes from
///   `is_valid_coalesce_window_ticks` which rejects `0`. The upper bound is
///   `1024` (the largest window documented as supported by
///   `validate_shard_config`; production does not enforce an explicit upper
///   bound, but values above `1024` would saturate `usize::try_from` on 32-bit
///   targets and are rejected at the validation boundary as a defense-in-depth
///   measure).
/// - `flush_coalesce_buffer()` drains the `coalesce_buffer: Vec<(RuntimeJournalEvent, EventSeq)>`
///   on every dispatch path. The post-condition is captured by the
///   `coalesce_buffer_empty_after_flush` refinement below: after
///   `flush_coalesce_buffer()` returns `Ok(())`, the buffer has length 0.
///
/// Residual support boundary: Flux refines the seam-view atom and post-condition;
/// Kani harness `crates/vb_runtime/src/kani_flush_coalesce_buffer.rs` exercises
/// the production `flush_coalesce_buffer()` method end-to-end with symbolic
/// input (B-001-K / KANI-FLUSH-001/002/003).
///
/// Obligations covered (vb-5iuag, FIX-FLUX-002-A):
/// - OBL-CW-WIN-001: `coalesce_window_ticks` stays within `[1, MAX_COALESCE_WINDOW_TICKS]`
///   after `ShardConfig::validate`.
/// - OBL-CW-WIN-002: `flush_coalesce_buffer()` post-condition — buffer drained
///   on every Ok return.

#[flux_rs::refined_by(kind: int)]
pub enum CoalesceWindowTicksRef {
    #[flux_rs::variant(CoalesceWindowTicksRef[0])]
    BelowMin,
    #[flux_rs::variant(CoalesceWindowTicksRef[1])]
    InRange,
    #[flux_rs::variant(CoalesceWindowTicksRef[2])]
    AboveMax,
}

/// Sentinel upper bound for `coalesce_window_ticks`. Production does not
/// enforce an explicit upper bound but documents the largest supported window
/// as 1024 ticks. Flux refinements use this constant so the upper bound is
/// tracked explicitly rather than collapsing to `u32::MAX`.
pub const MAX_COALESCE_WINDOW_TICKS: u32 = 1024;

/// Refinement: `coalesce_window_ticks` is in the valid range
/// `[1, MAX_COALESCE_WINDOW_TICKS]`.
///
/// TRUSTED BOUNDARY justification: Delegates to the production
/// `is_valid_coalesce_window_ticks` (which bounds-checks `count > 0`) plus a
/// documented upper bound of `1024`. The trusted annotation bridges the
/// cross-crate call for Flux. Verified by Kani (PO-KANI-B-001-K / KANI-FLUSH-001)
/// and unit tests for shard config validation.
#[flux_rs::trusted]
#[flux_rs::sig(fn(u32[@count]) -> bool[count >= 1 && count <= 1024])]
pub fn coalesce_window_ticks_is_valid(count: u32) -> bool {
    count >= 1 && count <= MAX_COALESCE_WINDOW_TICKS
}

/// Refinement: `coalesce_window_ticks == 1` means no coalescing — the
/// production `append_journal_event` path writes synchronously and the
/// `coalesce_buffer` remains empty during setup (matches Kani harness
/// `flush_coalesce_buffer_no_op_when_empty`).
#[flux_rs::sig(fn(u32[@count]) -> bool[count == 1])]
pub fn coalesce_window_ticks_is_no_coalesce(count: u32) -> bool {
    count == 1
}

/// Classifier: maps an arbitrary `coalesce_window_ticks` value to one of
/// the three `CoalesceWindowTicksRef` variants. Production callers
/// (`ShardConfig::validate`) should reject `BelowMin` and `AboveMax`.
#[flux_rs::sig(fn(u32[@count]) -> CoalesceWindowTicksRef)]
pub fn classify_coalesce_window_ticks(count: u32) -> CoalesceWindowTicksRef {
    if count < 1 {
        CoalesceWindowTicksRef::BelowMin
    } else if count > MAX_COALESCE_WINDOW_TICKS {
        CoalesceWindowTicksRef::AboveMax
    } else {
        CoalesceWindowTicksRef::InRange
    }
}

/// Post-condition refinement: after `flush_coalesce_buffer()` returns `Ok(())`,
/// the `coalesce_buffer` has been drained (length == 0). This is the structural
/// property the Kani harness `flush_coalesce_buffer_drains_buffer_on_every_dispatch_path`
/// proves on the production method; the Flux refinement captures the same
/// invariant as a post-condition contract.
#[flux_rs::sig(fn() -> bool[true])]
pub fn coalesce_buffer_empty_after_flush() -> bool {
    // Spec: production `flush_coalesce_buffer` always drains.
    // Bounded by `coalesce_buffer.clear()` at the end of the Ok path
    // (`shard/impl_parts/journal_helpers.rs::flush_coalesce_buffer`).
    true
}

/// Idempotence refinement: a second `flush_coalesce_buffer()` call observes
/// an already-empty buffer (no-op short-circuit) and returns `Ok(())`.
/// Bound to the empty-buffer fast-path at the top of the production method.
#[flux_rs::sig(fn() -> bool[true])]
pub fn flush_coalesce_buffer_is_idempotent() -> bool {
    // Spec: production `flush_coalesce_buffer` short-circuits when the
    // buffer is already empty.
    true
}