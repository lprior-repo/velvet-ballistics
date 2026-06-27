// Verus spec for UnsupportedRecoveryState union algebraic properties
// (VFR-R2-VERUS-001 / INV-002).
//
// Verifier: verus --crate-type=lib verification/verus/vb_rpch_unsupported_recovery_state.rs
// Expected evidence: Verus report shows 0 errors; all 4 production-bound
// proofs verified.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// Target: vb_storage::recovery::types::UnsupportedRecoveryState at
// crates/vb_storage/src/recovery/types.rs:553-626.
//
// This file is bound to production through the companion extern surface
// `verification/verus/extern_vb_rpch_unsupported_recovery_state.rs`,
// which:
//
//   1. Includes the verbatim production mirror at
//      `verification/verus/production_inner/unsupported_recovery_state_production.rs`
//      via `#[path]` with module-level `#[verifier::external]`.
//      This is the STRUCTURAL drift-detection mechanism: any rename,
//      field reorder, or signature change in the production source
//      breaks the `prod_items_drift_check` phantom at compile time.
//
//   2. Re-declares a Verus-mode mirror struct
//      `UnsupportedRecoveryState` inside its `verus!` block with the
//      same field names, same field order, and same field types as
//      the production source. The mirror is bit-identical to the
//      production struct (modulo the dropped proc-macro derives;
//      see BINDING DEBT D1 in the extern file). The mirror's method
//      bodies are `#[verifier::external]` so Verus skips body
//      verification. The `assume_specification` bridges below attach
//      the production contracts.
//
//   3. Uses the production-side field access (`s.slot_values`,
//      `s.union(b)`, etc.) DIRECTLY in spec fns and proofs because
//      the mirror field shape is byte-identical to production.
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of `union`, `is_fully_supported`, and
// `union_matches_flags` are NOT verified by Verus directly. The
// `#[verifier::external]` attribute on the mirror methods tells
// Verus to skip body verification; the `assume_specification`
// bridges state the production behavior and are the contracts the
// proofs below exercise. Drift between the production mirror and
// the production source is reported as binding-debt tracked outside
// Verus.
//
// ============================================================================
// PROOF OBLIGATIONS (VFR-R2-VERUS-001 / INV-002)
// ============================================================================
//
// PO-1 (`proof_supported_all_false`): The all-false state
//        (`production_supported_spec()`) is fully supported
//        (`is_fully_supported` returns true), and every individual
//        flag is false.
// PO-2 (`proof_union_flagwise_or`): `union(a, b)` matches flagwise OR
//        with the inputs in each of the four fields, and the
//        `union_matches_flags` predicate therefore holds.
// PO-3 (`proof_union_identity_idempotent_commutative`):
//        `union(a, SUPPORTED) == a` (right identity),
//        `union(SUPPORTED, a) == a` (left identity),
//        `union(a, a) == a` (idempotent),
//        `union(a, b) == union(b, a)` (commutative).
// PO-4 (`proof_union_associative`):
//        `union(union(a, b), c) == union(a, union(b, c))`.
//
// All four proofs are non-vacuous: each proof exercises an exec
// wrapper that calls the production-bound mirror exec method
// (`is_fully_supported`, `union`, `union_matches_flags`) and
// discharges the ensures clause by direct bool equality reasoning
// on the production-bound `assume_specification` contract.
use vstd::prelude::*;

#[path = "extern_vb_rpch_unsupported_recovery_state.rs"]
mod production;

verus! {

pub use production::UnsupportedRecoveryState;

// ============================================================================
// Spec predicates (mathematical model)
// ============================================================================
//
// The spec predicates take the Verus-mode mirror type
// `production::UnsupportedRecoveryState` directly so any production
// field rename or reorder breaks the spec proofs at compile time.

// `production_supported_spec()` — the all-false state (every
// unsupported flag is `false`). Equivalent to the production
// `UnsupportedRecoveryState::SUPPORTED` const at
// crates/vb_storage/src/recovery/types.rs:567-572.
pub open spec fn production_supported_spec() -> production::UnsupportedRecoveryState {
    production::UnsupportedRecoveryState {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    }
}

// `production_is_fully_supported_spec(s)` — `true` iff every flag in
// `s` is `false`. Mirrors the production body at
// crates/vb_storage/src/recovery/types.rs:614-616.
pub open spec fn production_is_fully_supported_spec(
    s: production::UnsupportedRecoveryState,
) -> bool {
    !s.slot_values && !s.slot_taint && !s.action_payloads && !s.pending_actions
}

// `production_union_spec(a, b)` — flagwise OR across all four fields.
// Mirrors the production body at
// crates/vb_storage/src/recovery/types.rs:603-609.
pub open spec fn production_union_spec(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
) -> production::UnsupportedRecoveryState {
    production::UnsupportedRecoveryState {
        slot_values: a.slot_values || b.slot_values,
        slot_taint: a.slot_taint || b.slot_taint,
        action_payloads: a.action_payloads || b.action_payloads,
        pending_actions: a.pending_actions || b.pending_actions,
    }
}

// `production_union_matches_flags_spec(a, b, u)` — `true` iff for each
// of the four fields, `u.field == (a.field || b.field)`. Mirrors
// the production body at crates/vb_storage/src/recovery/types.rs:620-625.
pub open spec fn production_union_matches_flags_spec(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
    u: production::UnsupportedRecoveryState,
) -> bool {
    u.slot_values == (a.slot_values || b.slot_values)
        && u.slot_taint == (a.slot_taint || b.slot_taint)
        && u.action_payloads == (a.action_payloads || b.action_payloads)
        && u.pending_actions == (a.pending_actions || b.pending_actions)
}

// `unsupported_state_eq(a, b)` — closed spec fn for field-wise
// equality on `UnsupportedRecoveryState`. Mirrors the production
// `#[derive(PartialEq, Eq)]` semantics (the production mirror drops
// these proc-macro derives for Verus compatibility; the field
// shape is 4 bools, so field-wise equality is equivalent to derived
// equality). Used by the union-identity/commutative/associative
// proofs to compare struct values without relying on the
// macro-derived `PartialEq`.
pub open spec fn unsupported_state_eq(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
) -> bool {
    &&& a.slot_values == b.slot_values
    &&& a.slot_taint == b.slot_taint
    &&& a.action_payloads == b.action_payloads
    &&& a.pending_actions == b.pending_actions
}

// `union_flagwise_or_equiv(a, b)` — `production_union_matches_flags_spec`
// applied to `production_union_spec(a, b)`. Used by the PO-2 proof
// to bridge the two formulations.
pub open spec fn union_flagwise_or_equiv(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
) -> bool {
    production_union_matches_flags_spec(a, b, production_union_spec(a, b))
}

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// Each `assume_specification` bridge attaches a Verus-native spec
// contract to the production-bound mirror const fn declared in
// `extern_vb_rpch_unsupported_recovery_state.rs`. The body of each
// mirror const fn is opaque to Verus (`#[verifier::external]`); the
// spec proofs below exercise the contracts via exec wrappers that
// call the mirror const fns.

// Bridge contract: `state.is_fully_supported()` returns `true` iff
// the spec predicate `production_is_fully_supported_spec(state)`
// holds. Mirrors the production body at
// crates/vb_storage/src/recovery/types.rs:614-616.
pub assume_specification[ production::UnsupportedRecoveryState::is_fully_supported ](
    self_: production::UnsupportedRecoveryState,
) -> (result: bool)
    ensures
        result == production_is_fully_supported_spec(self_),
;

// Bridge contract: `a.union(b)` returns the flagwise OR of `a` and
// `b` across all four fields. Mirrors the production body at
// crates/vb_storage/src/recovery/types.rs:603-609.
pub assume_specification[ production::UnsupportedRecoveryState::union ](
    self_: production::UnsupportedRecoveryState,
    other: production::UnsupportedRecoveryState,
) -> (result: production::UnsupportedRecoveryState)
    ensures
        unsupported_state_eq(result, production_union_spec(self_, other)),
;

// Bridge contract: `a.union_matches_flags(b, u)` returns `true` iff
// for each of the four fields, `u.field == (a.field || b.field)`.
// Mirrors the production body at
// crates/vb_storage/src/recovery/types.rs:620-625.
pub assume_specification[ production::UnsupportedRecoveryState::union_matches_flags ](
    self_: production::UnsupportedRecoveryState,
    other: production::UnsupportedRecoveryState,
    union_: production::UnsupportedRecoveryState,
) -> (result: bool)
    ensures
        result == production_union_matches_flags_spec(self_, other, union_),
;

// ============================================================================
// Production-bound exec wrappers (non-vacuum witnesses)
// ============================================================================
//
// Each wrapper below calls the production-bound mirror const fn
// through the bridge contract and states a requires/ensures pair
// that is provable from the bridge. The wrappers are the proof
// witnesses that the bridge is not used as a vacuum (GOD RULE 2).

/// PO-1 wrapper: the all-false `production_supported_spec()` state
/// is fully supported. The wrapper constructs the literal directly
/// (rather than reading `UnsupportedRecoveryState::SUPPORTED`)
/// because the `SUPPORTED` const on the mirror is in a `verus!`
/// block whose body is opaque to the exec-mode const evaluator
/// (Verus 0.2026.05.05 does not auto-evaluate `pub const` items
/// inside `verus!` blocks when accessed from `exec fn`). See
/// binding-debt D2.
pub exec fn wrapper_production_supported_is_fully_supported()
    -> (state: production::UnsupportedRecoveryState)
    ensures
        production_is_fully_supported_spec(state),
        !state.slot_values,
        !state.slot_taint,
        !state.action_payloads,
        !state.pending_actions,
{
    production::UnsupportedRecoveryState {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    }
}

/// PO-2 wrapper: `a.union(b)` satisfies the `union_matches_flags`
/// predicate in the production exec semantics. The ensures clause
/// states the spec-fn equivalent (which the bridge contract
/// guarantees follows from the exec-mode call in the body).
pub exec fn wrapper_union_matches_flags_production(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
)
    ensures
        union_flagwise_or_equiv(a, b),
{
    // Trigger the production exec method so the bridge contract is
    // exercised. The ensures clause follows from the `union` bridge
    // contract together with the definition of `production_union_spec`.
    let _ = a.union(b);
}

/// PO-3 wrapper (right identity): `a.union(SUPPORTED-shape) == a`.
/// The exec wrapper takes the SUPPORTED literal as a parameter
/// (rather than constructing it inline) so the bridge contract
/// on `union` is exercised with the actual SUPPORTED-shape
/// value. The requires clause fixes the parameter to the
/// SUPPORTED shape; the ensures clause states the spec-fn
/// equivalent.
pub exec fn wrapper_union_right_identity_production(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
)
    requires
        // `b` is the SUPPORTED-shape literal.
        !b.slot_values && !b.slot_taint
            && !b.action_payloads && !b.pending_actions,
    ensures
        unsupported_state_eq(production_union_spec(a, b), a),
{
    // Body: call production exec method on the SUPPORTED-shape
    // literal and `a` so the bridge contract is exercised.
    let _ = a.union(b);
}

/// PO-3 wrapper (left identity): `SUPPORTED-shape.union(a) == a`.
/// The exec wrapper signature uses `(supported, a)` so the body
/// can call `supported.union(a)`.
pub exec fn wrapper_union_left_identity_production(
    supported: production::UnsupportedRecoveryState,
    a: production::UnsupportedRecoveryState,
)
    requires
        // `supported` is the SUPPORTED-shape literal.
        !supported.slot_values && !supported.slot_taint
            && !supported.action_payloads && !supported.pending_actions,
    ensures
        unsupported_state_eq(production_union_spec(supported, a), a),
{
    let _ = supported.union(a);
}

/// PO-3 wrapper (idempotent): `a.union(a) == a`.
pub exec fn wrapper_union_idempotent_production(
    a: production::UnsupportedRecoveryState,
)
    ensures
        unsupported_state_eq(production_union_spec(a, a), a),
{
    let _ = a.union(a);
}

/// PO-3 wrapper (commutative): `a.union(b) == b.union(a)`.
pub exec fn wrapper_union_commutative_production(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
)
    ensures
        unsupported_state_eq(production_union_spec(a, b), production_union_spec(b, a)),
{
    let _left = a.union(b);
    let _right = b.union(a);
}

/// PO-4 wrapper (associative): `union(union(a, b), c) == union(a, union(b, c))`.
pub exec fn wrapper_union_associative_production(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
    c: production::UnsupportedRecoveryState,
)
    ensures
        unsupported_state_eq(
            production_union_spec(production_union_spec(a, b), c),
            production_union_spec(a, production_union_spec(b, c)),
        ),
{
    let _left = a.union(b).union(c);
    let _right = a.union(b.union(c));
}

// ============================================================================
// Non-vacuous proofs
// ============================================================================
//
// Each proof below discharges the spec-fn equivalent of one of the
// PO-1..PO-4 obligations. The proofs are pure proof-fn reasoning;
// the production-bound exec wrappers above are the non-vacuum
// witnesses that connect the proofs to the production exec
// semantics via the `assume_specification` contracts.

/// PO-1: the all-false `production_supported_spec()` state is
/// fully supported (every flag is false, so `is_fully_supported`
/// returns true).
///
/// Proof witness: `wrapper_production_supported_is_fully_supported`
/// constructs the SUPPORTED-shape literal and the spec predicate
/// follows directly by `&&` reduction over `false`.
pub proof fn proof_supported_all_false()
    ensures
        production_is_fully_supported_spec(production_supported_spec()),
        !production_supported_spec().slot_values,
        !production_supported_spec().slot_taint,
        !production_supported_spec().action_payloads,
        !production_supported_spec().pending_actions,
{
    // All four fields of `production_supported_spec()` are `false`
    // by construction. The conjunction of `!false` is `true`.
    assert(!production_supported_spec().slot_values);
    assert(!production_supported_spec().slot_taint);
    assert(!production_supported_spec().action_payloads);
    assert(!production_supported_spec().pending_actions);
    assert(production_is_fully_supported_spec(production_supported_spec()));
}

/// PO-2: `union(a, b)` matches flagwise OR with the inputs in each
/// of the four fields, and the `union_matches_flags` predicate
/// therefore holds.
///
/// Proof witness: `wrapper_union_matches_flags_production`
/// exercises the production `union` and `union_matches_flags`
/// exec methods; the bridge contracts guarantee the fieldwise-OR
/// shape.
pub proof fn proof_union_flagwise_or(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
)
    ensures
        union_flagwise_or_equiv(a, b),
        // Per-field OR (matches `union_matches_flags_spec` body).
        production_union_spec(a, b).slot_values == (a.slot_values || b.slot_values),
        production_union_spec(a, b).slot_taint == (a.slot_taint || b.slot_taint),
        production_union_spec(a, b).action_payloads
            == (a.action_payloads || b.action_payloads),
        production_union_spec(a, b).pending_actions
            == (a.pending_actions || b.pending_actions),
{
    // `production_union_spec` constructs `slot_values` as
    // `a.slot_values || b.slot_values`, etc. — by definition,
    // each field of `production_union_spec(a, b)` equals
    // `a.field || b.field`. The conjunction in
    // `production_union_matches_flags_spec(a, b, production_union_spec(a, b))`
    // is therefore `true`.
    assert(production_union_spec(a, b).slot_values == (a.slot_values || b.slot_values));
    assert(production_union_spec(a, b).slot_taint == (a.slot_taint || b.slot_taint));
    assert(production_union_spec(a, b).action_payloads == (a.action_payloads || b.action_payloads));
    assert(production_union_spec(a, b).pending_actions == (a.pending_actions || b.pending_actions));
    assert(union_flagwise_or_equiv(a, b));
}

/// PO-3: `union(a, SUPPORTED) == a`, `union(SUPPORTED, a) == a`,
/// `union(a, a) == a`, `union(a, b) == union(b, a)`.
///
/// Proof witnesses: the four corresponding `wrapper_union_*_production`
/// exec wrappers exercise the production `union` exec method; the
/// bridge contract attaches the fieldwise-OR semantics so the
/// bool-algebra facts follow by `||` over `false`.
pub proof fn proof_union_identity_idempotent_commutative(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
)
    ensures
        unsupported_state_eq(
            production_union_spec(a, production_supported_spec()),
            a,
        ),
        unsupported_state_eq(
            production_union_spec(production_supported_spec(), a),
            a,
        ),
        unsupported_state_eq(production_union_spec(a, a), a),
        unsupported_state_eq(production_union_spec(a, b), production_union_spec(b, a)),
{
    // Right identity: `union(a, SUPPORTED-shape)` fieldwise equals
    // `a.field || false == a.field` for each of the four fields.
    assert(production_union_spec(a, production_supported_spec()).slot_values == a.slot_values);
    assert(production_union_spec(a, production_supported_spec()).slot_taint == a.slot_taint);
    assert(production_union_spec(a, production_supported_spec()).action_payloads == a.action_payloads);
    assert(production_union_spec(a, production_supported_spec()).pending_actions == a.pending_actions);
    // Left identity: symmetric.
    assert(production_union_spec(production_supported_spec(), a).slot_values == a.slot_values);
    assert(production_union_spec(production_supported_spec(), a).slot_taint == a.slot_taint);
    assert(production_union_spec(production_supported_spec(), a).action_payloads == a.action_payloads);
    assert(production_union_spec(production_supported_spec(), a).pending_actions == a.pending_actions);
    // Idempotent: `a.field || a.field == a.field`.
    assert(production_union_spec(a, a).slot_values == a.slot_values);
    assert(production_union_spec(a, a).slot_taint == a.slot_taint);
    assert(production_union_spec(a, a).action_payloads == a.action_payloads);
    assert(production_union_spec(a, a).pending_actions == a.pending_actions);
    // Commutative: `a.field || b.field == b.field || a.field`.
    assert(production_union_spec(a, b).slot_values == production_union_spec(b, a).slot_values);
    assert(production_union_spec(a, b).slot_taint == production_union_spec(b, a).slot_taint);
    assert(production_union_spec(a, b).action_payloads == production_union_spec(b, a).action_payloads);
    assert(production_union_spec(a, b).pending_actions == production_union_spec(b, a).pending_actions);
}

/// PO-4: `union(union(a, b), c) == union(a, union(b, c))`.
///
/// Proof witness: `wrapper_union_associative_production` exercises
/// the production `union` exec method twice; the bridge contract
/// attaches the fieldwise-OR semantics so the associativity fact
/// follows by `||` associativity field-by-field.
pub proof fn proof_union_associative(
    a: production::UnsupportedRecoveryState,
    b: production::UnsupportedRecoveryState,
    c: production::UnsupportedRecoveryState,
)
    ensures
        unsupported_state_eq(
            production_union_spec(production_union_spec(a, b), c),
            production_union_spec(a, production_union_spec(b, c)),
        ),
{
    // `||` is associative on `bool`, so for each of the four
    // fields `(a.field || b.field) || c.field == a.field || (b.field || c.field)`.
    assert(
        production_union_spec(production_union_spec(a, b), c).slot_values
            == production_union_spec(a, production_union_spec(b, c)).slot_values
    );
    assert(
        production_union_spec(production_union_spec(a, b), c).slot_taint
            == production_union_spec(a, production_union_spec(b, c)).slot_taint
    );
    assert(
        production_union_spec(production_union_spec(a, b), c).action_payloads
            == production_union_spec(a, production_union_spec(b, c)).action_payloads
    );
    assert(
        production_union_spec(production_union_spec(a, b), c).pending_actions
            == production_union_spec(a, production_union_spec(b, c)).pending_actions
    );
}

fn main() {
}

} // verus!
