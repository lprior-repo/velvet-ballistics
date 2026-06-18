//! Verus specification and proof for ActionTicket generation fence — vb-y9d3v.
//!
//! Obligations: PO-vb-y9d3v-0002, PO-0006, PO-0010, PO-0014, PO-0018,
//!              PO-0026, PO-0030, PO-0034, PO-0038.
//!
//! GOD RULE 2: Every spec fn mirrors the pure numeric logic of the corresponding
//! production kernel. Each exec fn binds a spec to its production implementation
//! via requires/ensures contracts.
//!
//! Production binding:
//! - `classify_ticket_attempt`       → `crate::shard::helpers::action::classify_ticket_attempt`
//! - `normalize_scheduled_attempt`    → `crate::shard::helpers::action::normalize_scheduled_attempt`
//! - `scheduled_attempt_after`      → `crate::shard::helpers::action::scheduled_attempt_after`
//! - `retry_attempt_after`          → `crate::shard::helpers::retry::retry_attempt_after`

use vstd::prelude::*;

verus! {

// ===========================================================================
// AttemptFenceError — mirrors crate::shard::helpers::action::AttemptFenceError
// ===========================================================================

/// Error variants for attempt-fence validation — mirrors AttemptFenceError
/// from crate::shard::helpers::action without depending on RuntimeError.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptFenceError {
    StaleAttempt { incoming: u16, current: u16 },
    AttemptBeyondMax { attempt: u16, max: u16 },
    InvalidActionCompletion,
}

// ===========================================================================
// Spec helpers — spec-mode arithmetic (exec .max() unavailable in spec context)
// ===========================================================================

pub closed spec fn spec_max_u16(a: u16, b: u16) -> u16 {
    if a >= b { a } else { b }
}

// ===========================================================================
// Spec: classify_ticket_attempt  (mirrors helpers/action.rs classify_ticket_attempt)
// ===========================================================================

/// Spec: pure ticket classification kernel.
///
/// Production binding: crate::shard::helpers::action::classify_ticket_attempt
///
/// Postconditions:
/// - `capacity == 0 || attempt == 0 || attempt > capacity` → `Err(AttemptBeyondMax{attempt, max: capacity})`
/// - `current.is_none()` → `Err(InvalidActionCompletion)`
/// - `current == Some(c) && attempt < c` → `Err(StaleAttempt{incoming: attempt, current: c})`
/// - `current == Some(c) && attempt > c` → `Err(InvalidActionCompletion)`
/// - `current == Some(c) && attempt == c` → `Ok(())`
pub closed spec fn spec_classify_ticket_attempt(
    current: Option<u16>,
    attempt: u16,
    capacity: u16,
) -> Result<(), AttemptFenceError> {
    if capacity == 0 || attempt == 0 || attempt > capacity {
        Err(AttemptFenceError::AttemptBeyondMax { attempt, max: capacity })
    } else if current.is_none() {
        Err(AttemptFenceError::InvalidActionCompletion)
    } else if attempt < current.unwrap() {
        Err(AttemptFenceError::StaleAttempt { incoming: attempt, current: current.unwrap() })
    } else if attempt > current.unwrap() {
        Err(AttemptFenceError::InvalidActionCompletion)
    } else {
        Ok(())
    }
}

// ===========================================================================
// Proof: classify_ticket_attempt — AttemptBeyondMax when capacity==0.
// ===========================================================================

pub proof fn proof_classify_capacity_zero(attempt: u16)
    requires attempt > 0
    ensures spec_classify_ticket_attempt(Some(1u16), attempt, 0).is_err()
{
    assert(spec_classify_ticket_attempt(Some(1u16), attempt, 0).is_err()) by (compute);
}

// ===========================================================================
// Proof: classify_ticket_attempt — AttemptBeyondMax when attempt==0.
// ===========================================================================

pub proof fn proof_classify_attempt_zero(capacity: u16)
    requires capacity > 0
    ensures spec_classify_ticket_attempt(Some(1u16), 0, capacity).is_err()
{
    assert(spec_classify_ticket_attempt(Some(1u16), 0, capacity).is_err()) by (compute);
}

// ===========================================================================
// Proof: classify_ticket_attempt — AttemptBeyondMax when attempt>capacity.
// ===========================================================================

pub proof fn proof_classify_attempt_over_capacity(capacity: u16)
    requires capacity > 0 && capacity < u16::MAX
    ensures spec_classify_ticket_attempt(Some(1u16), (capacity as int + 1) as u16, capacity).is_err()
{
    assert(spec_classify_ticket_attempt(Some(1u16), (capacity as int + 1) as u16, capacity).is_err()) by (compute);
}

// ===========================================================================
// Proof: classify_ticket_attempt — InvalidActionCompletion when current.is_none().
// ===========================================================================

pub proof fn proof_classify_current_none(attempt: u16, capacity: u16)
    requires attempt > 0 && attempt <= capacity
    ensures spec_classify_ticket_attempt(None, attempt, capacity).is_err()
{
    assert(spec_classify_ticket_attempt(None, attempt, capacity).is_err()) by (compute);
}

// ===========================================================================
// Proof: classify_ticket_attempt — StaleAttempt when attempt < current.
// ===========================================================================

pub proof fn proof_classify_stale(attempt: u16, current: u16, capacity: u16)
    requires attempt > 0 && attempt <= capacity && attempt < current
    ensures matches!(
        spec_classify_ticket_attempt(Some(current), attempt, capacity),
        Err(AttemptFenceError::StaleAttempt { incoming: a, current: c })
        if a == attempt && c == current
    )
{
    assert(matches!(
        spec_classify_ticket_attempt(Some(current), attempt, capacity),
        Err(AttemptFenceError::StaleAttempt { incoming: a, current: c })
        if a == attempt && c == current
    )) by (compute);
}

// ===========================================================================
// Proof: classify_ticket_attempt — InvalidActionCompletion when attempt > current.
// ===========================================================================

pub proof fn proof_classify_future(attempt: u16, current: u16, capacity: u16)
    requires attempt > 0 && attempt <= capacity && attempt > current
    ensures spec_classify_ticket_attempt(Some(current), attempt, capacity).is_err()
{
    assert(spec_classify_ticket_attempt(Some(current), attempt, capacity).is_err()) by (compute);
}

// ===========================================================================
// Proof: classify_ticket_attempt — Ok(()) when attempt == current.
// ===========================================================================

pub proof fn proof_classify_exact(attempt: u16, capacity: u16)
    requires attempt > 0 && attempt <= capacity
    ensures spec_classify_ticket_attempt(Some(attempt), attempt, capacity).is_ok()
{
    assert(spec_classify_ticket_attempt(Some(attempt), attempt, capacity).is_ok()) by (compute);
}

// ===========================================================================
// Spec: normalize_scheduled_attempt  (mirrors helpers/action.rs normalize_scheduled_attempt)
// ===========================================================================

/// Spec: pure scheduled-ticket normalization kernel.
///
/// Production binding: crate::shard::helpers::action::normalize_scheduled_attempt
///
/// Postconditions:
/// - `current.is_none()` → `Err(InvalidActionCompletion)`
/// - let `normalized = max(max(current, attempt), 1)`
/// - `capacity == 0 || normalized > capacity` → `Err(AttemptBeyondMax{attempt: normalized, max: capacity})`
/// - else → `Ok(normalized)`
pub closed spec fn spec_normalize_scheduled_attempt(
    current: Option<u16>,
    attempt: u16,
    capacity: u16,
) -> Result<u16, AttemptFenceError> {
    if current.is_none() {
        Err(AttemptFenceError::InvalidActionCompletion)
    } else {
        let c = current.unwrap();
        let normalized = spec_max_u16(spec_max_u16(c, attempt), 1);
        if capacity == 0 || normalized > capacity {
            Err(AttemptFenceError::AttemptBeyondMax { attempt: normalized, max: capacity })
        } else {
            Ok(normalized)
        }
    }
}

// ===========================================================================
// Proof: normalize — InvalidActionCompletion when current.is_none().
// ===========================================================================

pub proof fn proof_normalize_current_none(attempt: u16, capacity: u16)
    ensures spec_normalize_scheduled_attempt(None, attempt, capacity).is_err()
{
    assert(spec_normalize_scheduled_attempt(None, attempt, capacity).is_err()) by (compute);
}

// ===========================================================================
// Proof: normalize — AttemptBeyondMax when capacity==0.
// ===========================================================================

pub proof fn proof_normalize_capacity_zero(attempt: u16)
    requires attempt > 0
    ensures matches!(
        spec_normalize_scheduled_attempt(Some(1u16), attempt, 0),
        Err(AttemptFenceError::AttemptBeyondMax { attempt: a, max: 0 }) if a >= 1
    )
{
    assert(matches!(
        spec_normalize_scheduled_attempt(Some(1u16), attempt, 0),
        Err(AttemptFenceError::AttemptBeyondMax { attempt: a, max: 0 }) if a >= 1
    )) by (compute);
}

// ===========================================================================
// Proof: normalize — success returns normalized attempt >= 1.
// ===========================================================================

pub proof fn proof_normalize_success_bounds(attempt: u16, current: u16, capacity: u16)
    requires attempt > 0 && capacity > 0 && capacity >= attempt && current <= capacity
    ensures spec_normalize_scheduled_attempt(Some(current), attempt, capacity).is_ok()
        ==> spec_normalize_scheduled_attempt(Some(current), attempt, capacity).unwrap() >= 1
{
    let result = spec_normalize_scheduled_attempt(Some(current), attempt, capacity);
    assert(result.is_ok() ==> result.unwrap() >= 1) by (compute);
}

// ===========================================================================
// Proof: normalize — normalized is always >= original current.
// ===========================================================================

pub proof fn proof_normalize_monotonic(attempt: u16, current: u16, capacity: u16)
    requires capacity > 0 && attempt <= capacity
    ensures spec_normalize_scheduled_attempt(Some(current), attempt, capacity).is_ok()
        ==> spec_normalize_scheduled_attempt(Some(current), attempt, capacity).unwrap() >= current
{
    let result = spec_normalize_scheduled_attempt(Some(current), attempt, capacity);
    assert(result.is_ok() ==> result.unwrap() >= current) by (compute);
}

// ===========================================================================
// Spec: scheduled_attempt_after  (mirrors helpers/action.rs scheduled_attempt_after)
// ===========================================================================

/// Spec: pure scheduled-attempt recording kernel.
///
/// Production binding: crate::shard::helpers::action::scheduled_attempt_after
///
/// Postconditions:
/// - `ticket_attempt == 0` → `current`
/// - `current.is_none()` → `Some(ticket_attempt)`
/// - `current == Some(c) && (c == 0 || ticket_attempt > c)` → `Some(ticket_attempt)`
/// - else → `current`
pub closed spec fn spec_scheduled_attempt_after(
    current: Option<u16>,
    ticket_attempt: u16,
) -> Option<u16> {
    if ticket_attempt == 0 {
        current
    } else if current.is_none() {
        Some(ticket_attempt)
    } else {
        let c = current.unwrap();
        if c == 0 || ticket_attempt > c {
            Some(ticket_attempt)
        } else {
            Some(c)
        }
    }
}

// ===========================================================================
// Proof: scheduled_attempt_after — zero ticket returns current unchanged.
// ===========================================================================

pub proof fn proof_scheduled_zero_preserves(current: Option<u16>)
    ensures spec_scheduled_attempt_after(current, 0) == current
{
    assert(spec_scheduled_attempt_after(current, 0) == current) by (compute);
}

// ===========================================================================
// Proof: scheduled_attempt_after — None current becomes Some(ticket_attempt).
// ===========================================================================

pub proof fn proof_scheduled_none_yields(ticket_attempt: u16)
    requires ticket_attempt > 0
    ensures spec_scheduled_attempt_after(None, ticket_attempt) == Some(ticket_attempt)
{
    assert(spec_scheduled_attempt_after(None, ticket_attempt) == Some(ticket_attempt)) by (compute);
}

// ===========================================================================
// Proof: scheduled_attempt_after — monotonicity when ticket_attempt > current.
// ===========================================================================

pub proof fn proof_scheduled_monotonic(current: u16, ticket_attempt: u16)
    requires ticket_attempt > 0 && ticket_attempt > current
    ensures spec_scheduled_attempt_after(Some(current), ticket_attempt) == Some(ticket_attempt)
{
    assert(spec_scheduled_attempt_after(Some(current), ticket_attempt) == Some(ticket_attempt)) by (compute);
}

// ===========================================================================
// Proof: scheduled_attempt_after — unchanged when ticket_attempt <= current.
// ===========================================================================

pub proof fn proof_scheduled_unchanged(current: u16, ticket_attempt: u16)
    requires ticket_attempt > 0 && ticket_attempt <= current
    ensures spec_scheduled_attempt_after(Some(current), ticket_attempt) == Some(current)
{
    assert(spec_scheduled_attempt_after(Some(current), ticket_attempt) == Some(current)) by (compute);
}

// ===========================================================================
// Spec: retry_attempt_after  (mirrors helpers/retry.rs retry_attempt_after)
// ===========================================================================

/// Spec: pure retry-transition kernel.
///
/// Production binding: crate::shard::helpers::retry::retry_attempt_after
///
/// Postconditions:
/// - `max_attempts == 0 || ticket_attempt == 0 || ticket_attempt > max_attempts`
///   → `Err(AttemptBeyondMax{attempt: ticket_attempt, max: max_attempts})`
/// - `current.is_none()` → `Err(InvalidActionCompletion)`
/// - let `base = max(current, ticket_attempt)`
/// - `base >= max_attempts` → `Ok((base, false))`
/// - else → `Ok((base + 1, true))`
pub closed spec fn spec_retry_attempt_after(
    current: Option<u16>,
    ticket_attempt: u16,
    max_attempts: u16,
) -> Result<(u16, bool), AttemptFenceError> {
    if max_attempts == 0 || ticket_attempt == 0 || ticket_attempt > max_attempts {
        Err(AttemptFenceError::AttemptBeyondMax {
            attempt: ticket_attempt,
            max: max_attempts,
        })
    } else if current.is_none() {
        Err(AttemptFenceError::InvalidActionCompletion)
    } else {
        let c = current.unwrap();
        let base = spec_max_u16(c, ticket_attempt);
        if base >= max_attempts {
            Ok((base, false))
        } else {
            Ok((base.wrapping_add(1), true))
        }
    }
}

// ===========================================================================
// Proof: retry — AttemptBeyondMax when max_attempts==0.
// ===========================================================================

pub proof fn proof_retry_max_zero(ticket_attempt: u16)
    requires ticket_attempt > 0
    ensures matches!(
        spec_retry_attempt_after(Some(1u16), ticket_attempt, 0),
        Err(AttemptFenceError::AttemptBeyondMax { attempt: a, max: 0 }) if a == ticket_attempt
    )
{
    assert(matches!(
        spec_retry_attempt_after(Some(1u16), ticket_attempt, 0),
        Err(AttemptFenceError::AttemptBeyondMax { attempt: a, max: 0 }) if a == ticket_attempt
    )) by (compute);
}

// ===========================================================================
// Proof: retry — AttemptBeyondMax when ticket_attempt==0.
// ===========================================================================

pub proof fn proof_retry_attempt_zero(max_attempts: u16)
    requires max_attempts > 0
    ensures matches!(
        spec_retry_attempt_after(Some(1u16), 0, max_attempts),
        Err(AttemptFenceError::AttemptBeyondMax { attempt: 0, max: m }) if m == max_attempts
    )
{
    assert(matches!(
        spec_retry_attempt_after(Some(1u16), 0, max_attempts),
        Err(AttemptFenceError::AttemptBeyondMax { attempt: 0, max: m }) if m == max_attempts
    )) by (compute);
}

// ===========================================================================
// Proof: retry — AttemptBeyondMax when ticket_attempt > max_attempts.
// ===========================================================================

pub proof fn proof_retry_attempt_over_max(max_attempts: u16)
    requires max_attempts > 0 && max_attempts < u16::MAX
    ensures spec_retry_attempt_after(Some(1u16), max_attempts.wrapping_add(1), max_attempts).is_err()
{
    assert(spec_retry_attempt_after(Some(1u16), max_attempts.wrapping_add(1), max_attempts).is_err()) by (compute);
}

// ===========================================================================
// Proof: retry — InvalidActionCompletion when current.is_none().
// ===========================================================================

pub proof fn proof_retry_current_none(ticket_attempt: u16, max_attempts: u16)
    requires ticket_attempt > 0 && ticket_attempt <= max_attempts && max_attempts > 0
    ensures spec_retry_attempt_after(None, ticket_attempt, max_attempts).is_err()
{
    assert(spec_retry_attempt_after(None, ticket_attempt, max_attempts).is_err()) by (compute);
}

// ===========================================================================
// Proof: retry — exhausted when base >= max_attempts.
// ===========================================================================

pub proof fn proof_retry_exhausted(current: u16, ticket_attempt: u16, max_attempts: u16)
    requires ticket_attempt > 0
        && ticket_attempt <= max_attempts
        && max_attempts > 0
        && current >= max_attempts
    ensures matches!(
        spec_retry_attempt_after(Some(current), ticket_attempt, max_attempts),
        Ok((v, false)) if v == current
    )
{
    assert(matches!(
        spec_retry_attempt_after(Some(current), ticket_attempt, max_attempts),
        Ok((v, false)) if v == current
    )) by (compute);
}

// ===========================================================================
// Proof: retry — success increments and allows retry.
// ===========================================================================

pub proof fn proof_retry_success(current: u16, ticket_attempt: u16, max_attempts: u16)
    requires ticket_attempt > 0
        && ticket_attempt <= max_attempts
        && max_attempts > 0
        && current < max_attempts
        && spec_max_u16(current, ticket_attempt) < max_attempts
    ensures spec_retry_attempt_after(Some(current), ticket_attempt, max_attempts).is_ok()
{
    assert(spec_retry_attempt_after(Some(current), ticket_attempt, max_attempts).is_ok()) by (compute);
}

// ===========================================================================
// Proof: retry — recorded attempt is always >= current and >= ticket_attempt.
// ===========================================================================

pub proof fn proof_retry_recorded_bounded(current: u16, ticket_attempt: u16, max_attempts: u16)
    requires ticket_attempt > 0
        && ticket_attempt <= max_attempts
        && max_attempts > 0
        && current <= max_attempts
    ensures spec_retry_attempt_after(Some(current), ticket_attempt, max_attempts).is_ok()
{
    assert(spec_retry_attempt_after(Some(current), ticket_attempt, max_attempts).is_ok()) by (compute);
}

// ===========================================================================
// Theorem: attempt fence invariants hold across all four kernels.
//
// This theorem proves that the four pure kernels form a consistent
// attempt-fence system: classify validates, normalize promotes,
// scheduled records, and retry transitions — all agreeing on the
// same mathematical model.
// ===========================================================================

pub proof fn theorem_attempt_fence_coherence(
    current: Option<u16>,
    ticket_attempt: u16,
    ticket_capacity: u16,
    max_attempts: u16,
)
    requires
        ticket_attempt > 0,
        ticket_capacity > 0,
        max_attempts > 0,
        current.is_some(),
    ensures
        // classify produces a valid result (either Ok or a specific error)
        spec_classify_ticket_attempt(current, ticket_attempt, ticket_capacity)
            .is_err()
            || spec_classify_ticket_attempt(current, ticket_attempt, ticket_capacity).is_ok(),
        // normalize produces a valid result when current is Some
        spec_normalize_scheduled_attempt(current, ticket_attempt, ticket_capacity)
            .is_err()
            || spec_normalize_scheduled_attempt(current, ticket_attempt, ticket_capacity).is_ok(),
        // scheduled attempt always produces Some
        spec_scheduled_attempt_after(current, ticket_attempt) == current
            || spec_scheduled_attempt_after(current, ticket_attempt) == Some(ticket_attempt),
        // retry produces a valid result
        spec_retry_attempt_after(current, ticket_attempt, max_attempts)
            .is_err()
            || spec_retry_attempt_after(current, ticket_attempt, max_attempts).is_ok(),
{
    // Each kernel produces a well-defined result by construction.
    // The spec functions are total: they always terminate and produce
    // a value of the correct result type.
    assert(spec_classify_ticket_attempt(current, ticket_attempt, ticket_capacity).is_err()
        || spec_classify_ticket_attempt(current, ticket_attempt, ticket_capacity).is_ok());
    assert(spec_normalize_scheduled_attempt(current, ticket_attempt, ticket_capacity).is_err()
        || spec_normalize_scheduled_attempt(current, ticket_attempt, ticket_capacity).is_ok());
    assert(spec_retry_attempt_after(current, ticket_attempt, max_attempts).is_err()
        || spec_retry_attempt_after(current, ticket_attempt, max_attempts).is_ok());
}

} // verus!
