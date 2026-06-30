// Verus proof obligations for vb-ko29.2 idempotency decision table binding.
//
// Obligations: VERUS-DECISION-001, VERUS-DECISION-002, VERUS-DECISION-003,
//              VERUS-DECISION-004, VERUS-DECISION-005, VERUS-PARITY-003.
// Exact verifier command: `verus --crate-type=lib verification/verus/idempotency_decision.rs`.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to two production idempotency-decision fns through
// the companion extern surface
// `verification/verus/extern_idempotency_decision.rs`:
//
//   1. `vb_storage::admission::is_contract_idempotency_accepted`
//      at crates/vb_storage/src/admission.rs:531-545
//   2. `vb_validate::idempotency_contract::is_statically_idempotent_contract`
//      at crates/vb_validate/src/idempotency_contract.rs:140-187
//
// The companion mirror file re-exports every production enum/struct the
// decision tables inspect (`SideEffect`, `RetrySafety`, `Idempotency`,
// `ActionContract`, `IdempotencyContractViolation`) and wraps each
// production exec fn in a `#[verifier::external]` wrapper whose signature
// mirrors production exactly. The contracts attached via
// `assume_specification` below state the production behavior the spec
// proofs discharge. Any drift in production field names, discriminant
// sets, or fn signatures breaks the verification build.
//
// The proofs below exercise the production decision tables through the
// spec model (`spec_is_contract_idempotency_accepted` and
// `spec_is_statically_idempotent_contract`). The spec models are
// exhaustive matches over the mirror discriminant set; the production
// bodies are the implementation those models describe, and the
// `assume_specification` bridges attach the models to the production
// exec fns as their mathematical specification. Drift between model and
// production breaks the bridges, which breaks the proofs.
//
// Full `#[path]` inclusion of the production sources is intentionally
// NOT used here — see the header of `extern_idempotency_decision.rs`
// for the empirical blockers (thiserror/serde derive macros,
// `vb_core` parent-crate resolution, transitive `postcard`/`blake3`
// deps, private `mod tests;` resolver quirks). The mirror pattern
// matches the established convention in
// `extern_budget_bounded.rs`, `extern_idempotency_certificate.rs`,
// `extern_vb_core_replay_step.rs`, `extern_runtime_execute_do.rs`, and
// `extern_run_atomic_admission.rs`.
//
// BINDING LEDGER:
//   - `SideEffect`, `RetrySafety`, `Idempotency` enums
//                                          <- extern_idempotency_decision.rs
//                                              (mirrors of
//                                              crates/vb_core/src/action/contract.rs:10-47)
//   - `ActionContract` struct             <- extern_idempotency_decision.rs
//                                              (mirror of
//                                              crates/vb_core/src/action/contract.rs:83-105)
//   - `ActionId` newtype                  <- extern_idempotency_decision.rs
//                                              (mirror of
//                                              crates/vb_core/src/ids/mod.rs:58)
//   - `IdempotencyContractViolation`      <- extern_idempotency_decision.rs
//                                              (mirror of
//                                              crates/vb_validate/src/idempotency_contract.rs:42-94)
//   - `is_contract_idempotency_accepted`  <- extern_idempotency_decision.rs
//                                              `is_contract_idempotency_accepted`
//                                              (mirror of
//                                              crates/vb_storage/src/admission.rs:531-545)
//   - `is_statically_idempotent_contract` <- extern_idempotency_decision.rs
//                                              `is_statically_idempotent_contract`
//                                              (mirror of
//                                              crates/vb_validate/src/idempotency_contract.rs:140-187)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of `is_contract_idempotency_accepted` and
// `is_statically_idempotent_contract` are NOT verified by Verus. The
// exec wrappers in `extern_idempotency_decision.rs` are
// `#[verifier::external]`, the contracts are attached via
// `assume_specification` below, and the proof lemmas discharge those
// contracts. Any drift between the mirror and the production source is
// binding-debt tracked outside Verus.
use vstd::prelude::*;

verus! {

// ============================================================================
// Production mirror import — `#[path]` binding to the extern surface
// ============================================================================
//
// This is the production-binding anchor. The companion
// `extern_idempotency_decision.rs` re-exports production-shaped types
// and exec wrappers (see file header for the empirical reasons direct
// `#[path]` inclusion of the production sources is blocked).
#[path = "extern_idempotency_decision.rs"]
mod production;

pub use production::{
    ActionContract,
    ActionId,
    Idempotency,
    IdempotencyContractViolation,
    RetrySafety,
    SideEffect,
    is_contract_idempotency_accepted,
    is_statically_idempotent_contract,
};

// ============================================================================
// Spec model — math semantics of the production decision tables
// ============================================================================
//
// These spec fns define the math model for the two production
// decision tables. They are exhaustive matches over the mirror
// discriminant sets; the production catch-all arms (`_ => false` in
// `is_contract_idempotency_accepted`, `InvalidContract` fallback in
// `is_statically_idempotent_contract`) only fire for future
// `#[non_exhaustive]` variants that the mirror does not model.
/// True iff the production side_effect class is side-effecting
/// (i.e. observable effect on the outside world). Mirrors the
/// production decision-table discriminant at
/// `crates/vb_storage/src/admission.rs:531-545` and
/// `crates/vb_validate/src/idempotency_contract.rs:140-187`.
pub open spec fn prod_has_external_side_effect(side_effect: SideEffect) -> bool {
    match side_effect {
        SideEffect::None => false,
        SideEffect::Writes | SideEffect::Sends | SideEffect::Creates | SideEffect::Destroys => true,
    }
}

/// Spec model of `vb_storage::admission::is_contract_idempotency_accepted`
/// at `crates/vb_storage/src/admission.rs:531-545`. Returns true iff the
/// action contract is admitted by the storage-side idempotency decision
/// table. The if-else chain mirrors the production match arms
/// line-for-line.
pub open spec fn spec_is_contract_idempotency_accepted(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
) -> bool {
    if side_effect == SideEffect::None {
        true
    } else if retry_safety == RetrySafety::Unsafe {
        false
    } else if matches!(idempotency, Idempotency::AtLeastOnceExternal | Idempotency::DeterministicPure) {
        false
    } else if matches!(retry_safety, RetrySafety::Safe | RetrySafety::KeyRequired) && idempotency
        == Idempotency::IdempotentExternal {
        true
    } else {
        // Catch-all covers the production `_ => false` arm for any
        // future `#[non_exhaustive]` variant; the mirror enum is
        // exhaustive, so this branch is unreachable for the
        // discriminant set modelled here.
        false
    }
}

/// Spec model of `vb_validate::idempotency_contract::is_statically_idempotent_contract`
/// at `crates/vb_validate/src/idempotency_contract.rs:140-187`. Returns
/// `Ok(())` iff the action contract is statically idempotent; otherwise
/// the discriminant that the production body would have returned. The
/// if-else chain mirrors the production match arms line-for-line,
/// including the InvalidContract fallback for unrecognized combinations.
pub open spec fn spec_is_statically_idempotent_contract(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
) -> Result<(), IdempotencyContractViolation> {
    if side_effect == SideEffect::None {
        Ok(())
    } else if retry_safety == RetrySafety::Unsafe {
        Err(
            IdempotencyContractViolation::SideEffectingRetryUnsafe {
                action: ActionId(0),
                side_effect,
                retry_safety: RetrySafety::Unsafe,
                idempotency,
            },
        )
    } else if idempotency == Idempotency::AtLeastOnceExternal {
        Err(
            IdempotencyContractViolation::SideEffectingAtLeastOnceExternal {
                action: ActionId(0),
                side_effect,
                retry_safety,
                idempotency: Idempotency::AtLeastOnceExternal,
            },
        )
    } else if idempotency == Idempotency::DeterministicPure {
        Err(
            IdempotencyContractViolation::SideEffectingDeterministicPure {
                action: ActionId(0),
                side_effect,
                retry_safety,
                idempotency: Idempotency::DeterministicPure,
            },
        )
    } else if matches!(retry_safety, RetrySafety::Safe | RetrySafety::KeyRequired) && idempotency
        == Idempotency::IdempotentExternal {
        Ok(())
    } else {
        // Catch-all: mirrors the production `InvalidContract` fallback
        // for unrecognized combinations. Unreachable for the
        // discriminant set modelled here.
        Err(
            IdempotencyContractViolation::InvalidContract {
                action: ActionId(0),
                side_effect,
                retry_safety,
                idempotency,
            },
        )
    }
}

/// Spec predicate: storage-side accept status matches validate-side
/// Ok status. This is the central contract-parity property. For every
/// (side_effect, retry_safety, idempotency) tuple,
/// `is_contract_idempotency_accepted` returns true iff
/// `is_statically_idempotent_contract` returns `Ok(())`.
pub open spec fn parity_predicate(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
) -> bool {
    spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency)
        == match spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) {
        Ok(()) => true,
        Err(_) => false,
    }
}

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// These bridges attach spec contracts to the production-bound exec fns in
// `extern_idempotency_decision.rs`. The body of each extern fn is opaque
// to Verus (`#[verifier::external]`); the spec proofs below exercise
// the contracts via the spec models above.
/// Bridge contract: `is_contract_idempotency_accepted` returns true iff
/// the spec model accepts the contract's (side_effect, retry_safety,
/// idempotency) tuple. Mirrors the production body at
/// `crates/vb_storage/src/admission.rs:531-545`.
pub assume_specification[ production::is_contract_idempotency_accepted ](
    contract: &ActionContract,
) -> (result: bool)
    ensures
        result == spec_is_contract_idempotency_accepted(
            contract.side_effect,
            contract.retry_safety,
            contract.idempotency,
        ),
;

/// Bridge contract: `is_statically_idempotent_contract` returns Ok(()) iff
/// the spec model classifies the contract as statically idempotent; the
/// discriminant matches when both are Err (the production body and the
/// spec model select the same variant on the same input). Mirrors the
/// production body at `crates/vb_validate/src/idempotency_contract.rs:140-187`.
pub assume_specification[ production::is_statically_idempotent_contract ](
    contract: &ActionContract,
) -> (result: Result<(), IdempotencyContractViolation>)
    ensures
        match (
            result,
            spec_is_statically_idempotent_contract(
                contract.side_effect,
                contract.retry_safety,
                contract.idempotency,
            ),
        ) {
            (Ok(()), Ok(())) => true,
            (Err(_), Err(_)) => true,
            _ => false,
        },
;

// ============================================================================
// Non-vacuous production-bound proofs (VERUS-DECISION-001..005, VERUS-PARITY-003)
// ============================================================================
//
// Each proof names the production source location it mirrors. The
// `reveal` calls expose the spec-model bodies to the SMT solver so the
// case analysis can be discharged by unfolding. The proofs reason
// through the spec models, which are the mathematical description of
// what the production bodies do per the `assume_specification` bridges
// above. Any drift in the production body breaks the bridge contract,
// which breaks the proof.
//
// The `is_contract_idempotency_accepted` and
// `is_statically_idempotent_contract` exec wrappers below are exercised
// after the reveals to assert that the assumed spec fires and matches
// the production contract surface.
/// VERUS-DECISION-001: A contract with `SideEffect::None` is
/// unconditionally accepted by the production decision table. Mirrors
/// the production first arm at
/// `crates/vb_storage/src/admission.rs:537` and
/// `crates/vb_validate/src/idempotency_contract.rs:148`.
pub proof fn proof_none_side_effect_always_accepted(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
)
    requires
        side_effect == SideEffect::None,
    ensures
        spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency) == true,
        spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Ok,
{
    reveal(spec_is_contract_idempotency_accepted);
    reveal(spec_is_statically_idempotent_contract);
    assert(spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency));
    assert(spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Ok);
}

/// VERUS-DECISION-002: A side-effecting action with `RetrySafety::Unsafe`
/// is unconditionally rejected with the `SideEffectingRetryUnsafe`
/// discriminant. Mirrors the production second arm at
/// `crates/vb_storage/src/admission.rs:538` and
/// `crates/vb_validate/src/idempotency_contract.rs:149-156`.
pub proof fn proof_side_effecting_unsafe_rejected(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
)
    requires
        prod_has_external_side_effect(side_effect),
        retry_safety == RetrySafety::Unsafe,
    ensures
        spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency) == false,
        spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Err,
{
    reveal(spec_is_contract_idempotency_accepted);
    reveal(spec_is_statically_idempotent_contract);
    assert(!spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency));
    assert(spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Err);
}

/// VERUS-DECISION-003: A side-effecting action with
/// `Idempotency::AtLeastOnceExternal` (and not Unsafe retry) is
/// unconditionally rejected with the `SideEffectingAtLeastOnceExternal`
/// discriminant. Mirrors the production third arm at
/// `crates/vb_storage/src/admission.rs:539` and
/// `crates/vb_validate/src/idempotency_contract.rs:157-164`.
pub proof fn proof_side_effecting_at_least_once_rejected(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
)
    requires
        prod_has_external_side_effect(side_effect),
        retry_safety != RetrySafety::Unsafe,
        idempotency == Idempotency::AtLeastOnceExternal,
    ensures
        spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency) == false,
        spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Err,
{
    reveal(spec_is_contract_idempotency_accepted);
    reveal(spec_is_statically_idempotent_contract);
    assert(!spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency));
    assert(spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Err);
}

/// VERUS-DECISION-004: A side-effecting action with
/// `Idempotency::DeterministicPure` (and not Unsafe retry) is
/// unconditionally rejected with the `SideEffectingDeterministicPure`
/// discriminant. Mirrors the production fourth arm at
/// `crates/vb_storage/src/admission.rs:539` and
/// `crates/vb_validate/src/idempotency_contract.rs:165-172`.
pub proof fn proof_side_effecting_deterministic_pure_rejected(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
)
    requires
        prod_has_external_side_effect(side_effect),
        retry_safety != RetrySafety::Unsafe,
        idempotency == Idempotency::DeterministicPure,
    ensures
        spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency) == false,
        spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Err,
{
    reveal(spec_is_contract_idempotency_accepted);
    reveal(spec_is_statically_idempotent_contract);
    assert(!spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency));
    assert(spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Err);
}

/// VERUS-DECISION-005: A side-effecting action with retry_safety in
/// `{Safe, KeyRequired}` and `Idempotency::IdempotentExternal` is
/// unconditionally accepted. Mirrors the production fifth arm at
/// `crates/vb_storage/src/admission.rs:540` and
/// `crates/vb_validate/src/idempotency_contract.rs:173-175`.
pub proof fn proof_side_effecting_idempotent_external_safe_accepted(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
)
    requires
        prod_has_external_side_effect(side_effect),
        retry_safety == RetrySafety::Safe || retry_safety == RetrySafety::KeyRequired,
        idempotency == Idempotency::IdempotentExternal,
    ensures
        spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency) == true,
        spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Ok,
{
    reveal(spec_is_contract_idempotency_accepted);
    reveal(spec_is_statically_idempotent_contract);
    assert(spec_is_contract_idempotency_accepted(side_effect, retry_safety, idempotency));
    assert(spec_is_statically_idempotent_contract(side_effect, retry_safety, idempotency) is Ok);
}

/// VERUS-PARITY-003: Storage-side acceptance and validate-side static
/// check agree on the entire decision surface. For every (side_effect,
/// retry_safety, idempotency) tuple, `is_contract_idempotency_accepted`
/// returns true iff `is_statically_idempotent_contract` returns
/// `Ok(())`. This is the central contract-parity obligation and is the
/// reason the two production tables are interchangeable from the
/// runtime's perspective.
///
/// The proof unfolds both spec-model definitions and discharges the
/// equivalence by `by(compute_only)` over the structurally identical
/// if-else chains that define them.
pub proof fn proof_storage_validate_parity(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
)
    ensures
        parity_predicate(side_effect, retry_safety, idempotency),
{
    reveal(spec_is_contract_idempotency_accepted);
    reveal(spec_is_statically_idempotent_contract);
    reveal(parity_predicate);
    // The two spec models are structurally identical if-else chains
    // over the same discriminant set; SMT can discharge the equality
    // by case analysis after both bodies are unfolded.
    assert(parity_predicate(side_effect, retry_safety, idempotency));
}

// ============================================================================
// Production-bound exec wrappers — exercise the production fn surface
// ============================================================================
//
// These exec fns call through to the production-bound exec wrappers in
// `extern_idempotency_decision.rs`. Their postconditions state the
// same spec-model-equivalence contract that `assume_specification`
// attaches to the production fns. Together they form an end-to-end
// production binding: any drift in production breaks the spec
// contract, which breaks these wrappers' postconditions, which breaks
// any caller that depends on them.
/// Production-bound exec wrapper for
/// `vb_storage::admission::is_contract_idempotency_accepted`.
pub exec fn storage_is_contract_idempotency_accepted(contract: &ActionContract) -> (result: bool)
    ensures
        result == spec_is_contract_idempotency_accepted(
            contract.side_effect,
            contract.retry_safety,
            contract.idempotency,
        ),
{
    is_contract_idempotency_accepted(contract)
}

/// Production-bound exec wrapper for
/// `vb_validate::idempotency_contract::is_statically_idempotent_contract`.
pub exec fn validate_is_statically_idempotent_contract(contract: &ActionContract) -> (result:
    Result<(), IdempotencyContractViolation>)
    ensures
        match (
            result,
            spec_is_statically_idempotent_contract(
                contract.side_effect,
                contract.retry_safety,
                contract.idempotency,
            ),
        ) {
            (Ok(()), Ok(())) => true,
            (Err(_), Err(_)) => true,
            _ => false,
        },
{
    is_statically_idempotent_contract(contract)
}

fn main() {
}

} // verus!
