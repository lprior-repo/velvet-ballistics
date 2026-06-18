//! Taint propagation for actions based on idempotency classification.

use crate::value::Taint;
use crate::action::classification::Idempotency;

/// Computes the output taint for an action given its idempotency and input taint.
///
/// Rules:
/// - DeterministicPure and Idempotency::IdempotentExternal: output taint >= input taint (join).
/// - AtLeastOnceExternal: DerivedFromSecret when any input is Secret/DerivedFromSecret.
/// - Clean result from tainted input is rejected unless the action declares declassification
///   (not modeled here; caller must validate).
///
/// # Defense-in-depth note
///
/// This function is kept in sync with `vb_runtime::shard::lifecycle::reject_taint_downgrade`.
/// Both are defense-in-depth layers; the runtime enforces at completion and the core enforces
/// at validation. The duplication is architectural debt — do not refactor one without checking the other.
#[must_use]
pub const fn propagate_action_taint(idempotency: Idempotency, input_taint: Taint) -> Taint {
    match idempotency {
        // Deterministic/idempotent actions propagate taint unchanged (identity join).
        Idempotency::DeterministicPure | Idempotency::IdempotentExternal => input_taint,
        Idempotency::AtLeastOnceExternal => match input_taint {
            Taint::Clean => Taint::Clean,
            Taint::Secret | Taint::DerivedFromSecret => Taint::DerivedFromSecret,
        },
    }
}
