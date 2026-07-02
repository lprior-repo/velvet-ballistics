#![forbid(unsafe_code)]
//! Decision selection helpers for [`SeededScheduler`].
//!
//! This module owns the [`BoundaryDecision`] selection pipeline:
//! the [`FAIL_VARIANT_POOL`] constant, the [`TickOutcome`] enum,
//! and the free functions that translate scheduler state into a
//! decision variant or a [`ShardDirective`]. Splitting these out of
//! `decision.rs` keeps the public-API file under the 300-line
//! production ceiling while preserving the `pub(crate)` visibility
//! required for the main implementation module.

use crate::scheduler::types::{BoundaryChoice, BoundaryDecision, BoundaryPolicy};
use crate::shard::ShardDirective;
use vb_core::ids::StepIdx;

/// Pool of candidate [`crate::RuntimeError`] variants surfaced by a
/// `Fail` boundary decision under [`BoundaryPolicy::RoundRobin`] and
/// [`BoundaryPolicy::Random`].
///
/// Determinism: the same seed + same `decision_count` produces the
/// same variant because the selection is purely a function of the
/// scheduler's PRNG state (or its monotonic counter under round
/// robin). This eliminates the prior hard-coded single-variant surface
/// and increases divergence under exploration.
pub(crate) const FAIL_VARIANT_POOL: &[fn() -> crate::RuntimeError] = &[
    || crate::RuntimeError::ShutdownInProgress,
    || crate::RuntimeError::InvalidTimerFire,
    || crate::RuntimeError::InvalidRecoveryHydration,
    || crate::RuntimeError::UnsupportedFullRecoveryHydration,
    || crate::RuntimeError::FramePoolUnavailable,
];

/// Standalone fallback for the structurally-unreachable index-0 miss
/// path. Matches the first entry of [`FAIL_VARIANT_POOL`]. The
/// static lifetime ensures the reference can be returned safely
/// from a match arm.
static FIRST_POOL_ENTRY: fn() -> crate::RuntimeError = || crate::RuntimeError::ShutdownInProgress;

/// Outcome of a single scheduler tick attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickOutcome {
    /// The runtime accepted the tick and remains alive; keep going.
    Continue,
    /// The runtime reported natural completion (at least one shard
    /// shut down via `Runtime::tick_all` returning `Ok(false)`).
    Complete,
    /// The scheduler emitted a [`BoundaryDecision::Fail`]; stop with
    /// `RunEndReason::FailedDecision`.
    Fail,
}

/// Selects the decision variant for a `Free` choice under the
/// configured [`BoundaryPolicy`]. For `RoundRobin` and `Random` the
/// `StepIdx` target is derived from `decision_count` to keep outputs
/// bounded and deterministic.
///
/// This is a free function (rather than a method on
/// [`crate::scheduler::SeededScheduler`]) so it can be unit-tested
/// without constructing a scheduler. The PRNG cursor is threaded
/// through explicitly via `round_robin_cursor` to avoid mutating
/// scheduler state.
pub(crate) fn select_decision(
    choice: &BoundaryChoice,
    policy: BoundaryPolicy,
    decision_count: u32,
    rng_pick: u32,
    round_robin_cursor: u8,
) -> (BoundaryDecision, u8) {
    let decision = match (choice, policy) {
        // Constrained choices: the scheduler honours the caller's
        // restriction regardless of policy.
        (BoundaryChoice::AdvanceOnly, _) => BoundaryDecision::Advance,
        (BoundaryChoice::YieldOnly { to_step }, _) => BoundaryDecision::Yield { to_step: *to_step },
        (BoundaryChoice::FailOnly { variant }, _) => BoundaryDecision::Fail {
            variant: variant.clone(),
        },
        (BoundaryChoice::RetryOnly { delay_ticks }, _) => BoundaryDecision::Retry {
            delay_ticks: *delay_ticks,
        },
        // Free choice: policy selects the variant.
        (BoundaryChoice::Free, BoundaryPolicy::First) => BoundaryDecision::Advance,
        (BoundaryChoice::Free, BoundaryPolicy::RoundRobin) => {
            let variant = round_robin_cursor & 0b0000_0011;
            materialize_free_variant(decision_count, u32::from(variant))
        }
        (BoundaryChoice::Free, BoundaryPolicy::Random) => {
            materialize_free_variant(decision_count, rng_pick)
        }
    };
    // Advance the round-robin cursor only when the round-robin policy
    // is in use; other paths leave the cursor untouched.
    let next_cursor = match (choice, policy) {
        (BoundaryChoice::Free, BoundaryPolicy::RoundRobin) => round_robin_cursor.wrapping_add(1),
        _ => round_robin_cursor,
    };
    (decision, next_cursor)
}

/// Materialises a `Free` choice variant from an index in 0..=3.
/// Index 0 → Advance, 1 → Yield, 2 → Fail, 3 → Retry.
///
/// The `Fail` variant is selected from [`FAIL_VARIANT_POOL`]
/// deterministically using `decision_count` so that two schedulers
/// with the same seed and the same decision history emit the same
/// `RuntimeError` variant.
pub(crate) fn materialize_free_variant(decision_count: u32, variant: u32) -> BoundaryDecision {
    match variant {
        0 => BoundaryDecision::Advance,
        1 => BoundaryDecision::Yield {
            to_step: mask_to_step_idx(decision_count),
        },
        2 => fail_from_pool(decision_count),
        _ => BoundaryDecision::Retry {
            // `decision_count.saturating_add(1)` is in
            // `[0, u32::MAX]`. AND-masking with `0xFF` is bitwise
            // (no `as`, no arithmetic side-effect lint), and
            // yields a value in `[0, 255]`. The documented
            // contract for `delay_ticks` is a small bounded
            // non-negative integer.
            delay_ticks: decision_count.saturating_add(1) & 0xFF,
        },
    }
}

/// Selects a `Fail` variant from [`FAIL_VARIANT_POOL`] using
/// `decision_count` modulo the pool length. Extracted from
/// [`materialize_free_variant`] to keep that function under the
/// 25-line hot-function ceiling.
///
/// The selection is purely deterministic: two schedulers with the
/// same `decision_count` always pick the same `RuntimeError`
/// variant, regardless of policy.
fn fail_from_pool(decision_count: u32) -> BoundaryDecision {
    // PRNG-driven selection over `FAIL_VARIANT_POOL`.
    // `FAIL_VARIANT_POOL.len()` is a non-zero const slice (≥ 1
    // element), so `checked_rem` returns `Some` and the result
    // is in `[0, FAIL_VARIANT_POOL.len())`.
    let pool_len = u32::try_from(FAIL_VARIANT_POOL.len()).unwrap_or(1).max(1);
    #[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
    let pool_index_u32 = match decision_count.checked_rem(pool_len) {
        Some(v) => v,
        // `pool_len > 0`, so `checked_rem` always returns
        // `Some`. The `None` arm is structurally unreachable;
        // we pick 0 as the documented fallback.
        None => 0,
    };
    #[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
    let pool_index = match usize::try_from(pool_index_u32) {
        Ok(v) => v,
        // `pool_index_u32 < pool_len <= usize::MAX`. The `Err`
        // arm is structurally unreachable; we pick 0 as the
        // documented fallback.
        Err(_) => 0,
    };
    // Use `get` to satisfy `clippy::indexing_slicing`. The
    // double-match pattern (here + the inner `first()` re-fetch)
    // documents both structurally-unreachable fallbacks
    // without resorting to `expect()` or `unwrap()` (both
    // forbidden at the workspace level).
    let variant_ctor: &fn() -> crate::RuntimeError = match FAIL_VARIANT_POOL.get(pool_index) {
        Some(ctor) => ctor,
        None => {
            #[allow(clippy::get_first)]
            let first_entry = FAIL_VARIANT_POOL.first();
            match first_entry {
                Some(ctor) => ctor,
                // `FAIL_VARIANT_POOL` is non-empty by const
                // construction; re-borrow the static
                // `FIRST_POOL_ENTRY` defined at module scope.
                None => &FIRST_POOL_ENTRY,
            }
        }
    };
    BoundaryDecision::Fail {
        variant: variant_ctor(),
    }
}

/// Translates a [`BoundaryDecision`] into a [`ShardDirective`] for a
/// given shard, ensuring the directive's preconditions are satisfied
/// (in particular, `Migrate { target }` must point at a shard other
/// than the source).
///
/// This is a free function (not a method on the scheduler) so it
/// can be unit-tested without constructing a scheduler.
pub(crate) fn translate_decision_to_directive(
    decision: &BoundaryDecision,
    source_shard: u32,
    shard_count: u32,
) -> ShardDirective {
    match decision {
        BoundaryDecision::Advance => ShardDirective::Continue,
        BoundaryDecision::Yield { to_step } => {
            // Reduce the step index into a valid shard index space.
            // `to_step.get()` is a u16 (≤ 0xFFFF); `shard_count` is
            // at least 1 (validated at construction), so the modulo
            // is well-defined. We then ensure the target is not
            // the source shard (which `Runtime::tick_shard`
            // rejects with `MigrateSelf`); if the modulo
            // collapses to the source, advance by one position in
            // cyclic order.
            //
            // The `match` patterns below explicitly acknowledge the
            // structurally-unreachable fallback arms. Clippy's
            // `manual_unwrap_or` and `manual_unwrap_or_default`
            // lints would suggest `.unwrap_or(0)` or
            // `.unwrap_or_default()` instead, but
            // `unwrap_used = "forbid"` rejects those forms at the
            // workspace level; the explicit `match` is therefore
            // the right shape for this codebase. The
            // `#[allow(...)]` on each binding suppresses the
            // suggestions without weakening any other lint.
            let shard_count_safe = shard_count.max(1);
            #[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
            let raw = match u32::from(to_step.get()).checked_rem(shard_count_safe) {
                Some(v) => v,
                // `shard_count_safe > 0`, so `checked_rem` always
                // returns `Some`. The `None` arm is structurally
                // unreachable; we pick 0 as the documented
                // fallback.
                None => 0,
            };
            #[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
            let advanced = match raw
                .checked_add(1)
                .and_then(|v| v.checked_rem(shard_count_safe))
            {
                Some(v) => v,
                // `shard_count_safe > 0`, so `checked_rem` always
                // returns `Some` when `raw + 1` succeeds. The `None`
                // arm is structurally unreachable; we pick 0 as the
                // documented fallback.
                None => 0,
            };
            let target = if raw == source_shard { advanced } else { raw };
            ShardDirective::Migrate { target }
        }
        BoundaryDecision::Fail { .. } => ShardDirective::Shutdown,
        BoundaryDecision::Retry { .. } => ShardDirective::Suspend,
    }
}

/// Lossless conversion of a `u32` decision counter to a [`StepIdx`].
///
/// The bitwise `& 0xFFFF` mask narrows the value to exactly 16 bits,
/// guaranteeing the resulting value is in `[0, u16::MAX]`. The
/// `u16::try_from` therefore cannot fail; the `Err` arm is
/// structurally unreachable. We use an explicit `match` instead of
/// `as u16` so we never depend on lossy narrowing (clippy
/// `as_conversions` is denied workspace-wide) and we never return a
/// panic-shaped default from an impossible error path.
fn mask_to_step_idx(decision_count: u32) -> StepIdx {
    let masked = decision_count & 0xFFFF;
    match u16::try_from(masked) {
        Ok(v) => StepIdx::new(v),
        // `masked <= 0xFFFF = u16::MAX`, so this arm is
        // structurally unreachable. We pick `StepIdx::new(0)` as
        // the documented fallback for the impossible case rather
        // than propagate the error (which would force the caller
        // to convert the value into a typed error, increasing
        // type complexity without changing observable behaviour).
        Err(_) => StepIdx::new(0),
    }
}
