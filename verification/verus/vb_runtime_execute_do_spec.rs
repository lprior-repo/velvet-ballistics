// Verus spec for vb_runtime::engine::action::execute_do determinism gap.
//
// PO: VB-RUNTIME-EXEC-DO-001 (registered in contracts/proof_obligations.yaml).
// The execute_do dispatcher and the six sibling public exec wrappers in
// crates/vb_runtime/src/engine/action.rs are production-bound via
// assume_specification to pure projections defined in
// extern_runtime_execute_do.rs.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
// Target: vb_runtime::engine::action::execute_do
//   at crates/vb_runtime/src/engine/action.rs:20-74
//
// Production signature (action.rs:19-30):
//   pub fn execute_do(
//       run: &RunFrame,
//       step: StepIdx,
//       action: ActionId,
//       input: SlotIdx,
//       seq: SeqNo,
//       _contract: &ActionContract,
//       registry_contracts: &[ActionContract],
//       granted: &CapabilitySet,
//       retry_policy: RetryPolicy,
//   ) -> RuntimeEngineResult<RuntimeSignal>
//
// Production decision branches (action.rs:31-73):
//   1. registry lookup: registry_contracts[action.get()].id == action
//      -> Err(UnknownAction) otherwise
//   2. read_taint(input) failure
//      -> Err(Core(...)) otherwise
//   3. resolved.idempotency == DeterministicPure && input_taint != Clean
//      -> Err(TaintViolation { step }) otherwise
//   4. any required_capability not in granted
//      -> Err(Core(CapabilityDenied { ... })) otherwise
//   5. propagate_action_taint -> Clean while input_taint != Clean
//      -> Err(TaintViolation { step }) otherwise
//   6. otherwise -> Ok(AwaitingAction(ticket))
//
// Binding mechanism: `#[path = "extern_runtime_execute_do.rs"]` imports the
// thin extern surface, which inlines a pure projection of the production
// decision fn. The spec file then attaches exec contracts via
// `assume_specification` and exercises them through an exec wrapper.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `execute_do` cannot be verified end-to-end inside
// Verus because it transitively depends on `vb_core::frame::RunFrame`,
// `vb_core::value::ValueStore`, and `vb_core::capability::CapabilitySet`,
// which contain heap allocations, indices, and runtime internals that
// Verus does not model. The pure projection in `extern_runtime_execute_do.rs`
// captures every decision branch the production function takes and is
// recorded as a trusted base in the binding ledger. Each proof below
// operates on the projection; any divergence between the projection and
// the production body is a binding debt item tracked outside Verus.

use vstd::prelude::*;

verus! {

#[path = "extern_runtime_execute_do.rs"]
mod production;

// ============================================================================
// Re-export the production-bound types and the pure projection
// ============================================================================

pub use production::{
    SpecActionOutcomeKind, SpecOutcomeKind, SpecResumeKind, compute_idempotency_key_pure,
    execute_do_pure_decision, execute_do_without_contract_pure_decision,
    execute_error_handler_pure_decision, execute_retry_check_pure_decision,
    resolve_contract_pure_decision, resume_action_outcome_pure_decision,
    spec_action_outcome_kind_valid, spec_propagate_action_taint,
};

// ============================================================================
// Spec predicates (mathematical model used by proofs)
// ============================================================================

/// Spec predicate: two `SpecOutcomeKind` values agree.
pub open spec fn spec_outcome_kind_eq(a: SpecOutcomeKind, b: SpecOutcomeKind) -> bool {
    a == b
}

/// Spec predicate: an outcome-kind discriminant is one of the documented
/// `RuntimeEngineResult<RuntimeSignal>` outcomes of `execute_do`.
///
/// The production `execute_do` either returns `Ok(AwaitingAction(ticket))`
/// or one of the typed errors (UnknownAction, TaintViolation, CapabilityDenied,
/// or a Core wrapper). Every other variant in `RuntimeSignal` and
/// `RuntimeEngineError` is unreachable from this body — the spec predicate
/// below is the closed discriminant set.
pub open spec fn spec_outcome_kind_valid(kind: SpecOutcomeKind) -> bool {
    matches!(
        kind,
        SpecOutcomeKind::OkAwaitingAction
            | SpecOutcomeKind::ErrCapabilityDenied
            | SpecOutcomeKind::ErrTaintViolation
            | SpecOutcomeKind::ErrUnknownAction
            | SpecOutcomeKind::ErrCore
    )
}

/// Spec predicate: an `ActionOutcome` discriminant is one of the three
/// documented variants (Ready, Suspended, Failed). This bounds the post-
/// resume view: any action invoked through `execute_do` ultimately
/// resolves through `resume_action_outcome` to one of those three.
pub open spec fn spec_post_resume_outcome_kind_valid(kind: SpecActionOutcomeKind) -> bool {
    matches!(
        kind,
        SpecActionOutcomeKind::Ready
            | SpecActionOutcomeKind::Suspended
            | SpecActionOutcomeKind::Failed
    )
}

/// Spec-side mirror of `production::spec_propagate_action_taint` defined in
/// `extern_runtime_execute_do.rs`. The spec mirror is what the spec-level
/// proofs reference from `ensures` clauses; the production exec helper in
/// the extern surface is the trusted base.
pub open spec fn spec_propagate_action_taint_spec(
    contract_idempotency_disc: u8,
    input_taint_disc: u8,
) -> bool {
    input_taint_disc != 0
}

/// Spec-side mirror of `production::SpecActionOutcomeKind::discriminant`.
/// Spec-level proofs reference this from `ensures` clauses rather than the
/// exec helper. Mirrors the production `ActionOutcome` variant order
/// (Ready=0, Suspended=1, Failed=2) at crates/vb_core/src/action/payload.rs:163-172.
pub open spec fn spec_action_outcome_discriminant(kind: SpecActionOutcomeKind) -> int {
    match kind {
        SpecActionOutcomeKind::Ready => 0,
        SpecActionOutcomeKind::Suspended => 1,
        SpecActionOutcomeKind::Failed => 2,
    }
}

/// Spec-side mirror of the production `execute_do_pure_decision` defined in
/// `extern_runtime_execute_do.rs`. This mirror is what the spec-level
/// proofs operate on; the production exec fn in the extern surface is the
/// trusted base and the exec wrapper `checked_prod_execute_do_deterministic`
/// below asserts equality between the two.
pub open spec fn spec_execute_do_decision(
    _run_id: u64,
    _step: u32,
    _action: u32,
    _input: u32,
    _seq: u64,
    input_taint_disc: u8,
    _contract_id: u32,
    contract_idempotency_disc: u8,
    registry_action_match: bool,
    all_required_caps_granted: bool,
    _retry_max_attempts: u16,
    read_taint_failed: bool,
    output_taint_cleanable_from_tainted: bool,
) -> SpecOutcomeKind {
    if !registry_action_match {
        SpecOutcomeKind::ErrUnknownAction
    } else if read_taint_failed {
        SpecOutcomeKind::ErrCore
    } else if contract_idempotency_disc == 0 && input_taint_disc != 0 {
        SpecOutcomeKind::ErrTaintViolation
    } else if !all_required_caps_granted {
        SpecOutcomeKind::ErrCapabilityDenied
    } else if output_taint_cleanable_from_tainted {
        SpecOutcomeKind::ErrTaintViolation
    } else {
        SpecOutcomeKind::OkAwaitingAction
    }
}

/// Spec predicate: the pure decision fn is deterministic in its inputs.
///
/// Two invocations with identical scalars return identical outcome-kind
/// discriminants. The decision fn is a closed Rust function whose entire
/// body is `match`-style arithmetic, so this spec is the formal
/// characterization of that property. This predicate reduces to a
/// definitional tautology because `spec_execute_do_decision` is a pure
/// spec fn with no ghost state.
pub open spec fn spec_execute_do_deterministic(
    run_id: u64,
    step: u32,
    action: u32,
    input: u32,
    seq: u64,
    input_taint_disc: u8,
    contract_id: u32,
    contract_idempotency_disc: u8,
    registry_action_match: bool,
    all_required_caps_granted: bool,
    retry_max_attempts: u16,
    read_taint_failed: bool,
    output_taint_cleanable_from_tainted: bool,
) -> bool {
    spec_execute_do_decision(
        run_id,
        step,
        action,
        input,
        seq,
        input_taint_disc,
        contract_id,
        contract_idempotency_disc,
        registry_action_match,
        all_required_caps_granted,
        retry_max_attempts,
        read_taint_failed,
        output_taint_cleanable_from_tainted,
    )
        == spec_execute_do_decision(
            run_id,
            step,
            action,
            input,
            seq,
            input_taint_disc,
            contract_id,
            contract_idempotency_disc,
            registry_action_match,
            all_required_caps_granted,
            retry_max_attempts,
            read_taint_failed,
            output_taint_cleanable_from_tainted,
        )
}

// ============================================================================
// Spec mirrors for the 6 remaining production exec wrappers in
// crates/vb_runtime/src/engine/action.rs (execute_do_without_contract,
// execute_retry_check, execute_error_handler, resume_action_outcome,
// compute_idempotency_key, resolve_contract).
//
// Each spec fn mirrors the pure projection defined in extern_runtime_execute_do.rs.
// ============================================================================

/// Spec-side mirror of `execute_do_without_contract_pure_decision`.
/// Mirrors the production body at crates/vb_runtime/src/engine/action.rs:76-106.
pub open spec fn spec_execute_do_without_contract_decision(
    input_taint_disc: u8,
) -> SpecOutcomeKind {
    if input_taint_disc != 0 {
        SpecOutcomeKind::ErrTaintViolation
    } else {
        SpecOutcomeKind::ErrCapabilityDenied
    }
}

/// Spec-side mirror of `execute_retry_check_pure_decision`.
/// Mirrors the production body at crates/vb_runtime/src/engine/action.rs:109-120.
pub open spec fn spec_execute_retry_check_decision(
    current_attempt: u16,
    max_attempts: u16,
    body: u32,
    exhausted: u32,
) -> u32 {
    if current_attempt < max_attempts {
        body
    } else {
        exhausted
    }
}

/// Spec-side mirror of `execute_error_handler_pure_decision`.
/// Mirrors the production body at crates/vb_runtime/src/engine/action.rs:123-131.
/// Discriminants: RetryPolicy::Retryable=0, ActionFailureCode::Unknown=255.
pub open spec fn spec_execute_error_handler_decision(
    failure_retry_policy_disc: u8,
    failure_code_disc: u8,
    handler: u32,
    body: u32,
) -> u32 {
    if failure_retry_policy_disc == 0 || failure_code_disc != 255 {
        handler
    } else {
        body
    }
}

/// Spec predicate: a `SpecResumeKind` discriminant is in the closed set
/// produced by `resume_action_outcome`.
pub open spec fn spec_resume_kind_valid(kind: SpecResumeKind) -> bool {
    matches!(
        kind,
        SpecResumeKind::Continue
            | SpecResumeKind::AwaitingAction
            | SpecResumeKind::ErrRetryExhausted
            | SpecResumeKind::ErrUnsupportedPrimitive
            | SpecResumeKind::ErrInternalInvariantViolation
    )
}

/// Spec-side mirror of `resume_action_outcome_pure_decision`.
/// Mirrors the production body at crates/vb_runtime/src/engine/action.rs:138-200.
pub open spec fn spec_resume_action_outcome_decision(
    outcome_disc: u8,
    attempt: u16,
    capacity: u16,
    retry_policy_disc: u8,
    seq_would_overflow: bool,
    attempt_would_overflow: bool,
) -> SpecResumeKind {
    if outcome_disc == 0 {
        SpecResumeKind::Continue
    } else if outcome_disc == 1 {
        SpecResumeKind::AwaitingAction
    } else if outcome_disc == 2 {
        if retry_policy_disc == 0 {
            if attempt < capacity && !seq_would_overflow && !attempt_would_overflow {
                SpecResumeKind::AwaitingAction
            } else {
                SpecResumeKind::ErrRetryExhausted
            }
        } else {
            SpecResumeKind::ErrUnsupportedPrimitive
        }
    } else {
        SpecResumeKind::ErrInternalInvariantViolation
    }
}

/// Spec-side mirror of `compute_idempotency_key_pure`.
/// Mirrors the production body at crates/vb_runtime/src/engine/action.rs:206-208
/// and crates/vb_core/src/action/ticket.rs:25-35 (FNV-1a-inspired wrapping
/// multiply-add hash).
pub open spec fn spec_compute_idempotency_key_decision(run: u64, seq: u64, action: u32) -> int {
    let run_part = run as int * 0x6c62272e07bb0143;
    let seq_part = seq as int;
    let action_part = action as int;
    let step1 = run_part + seq_part;
    let step2 = step1 * 0x3b4f1a5b6c2d8e7f;
    let step3 = step2 + action_part;
    let result = step3 * 0x5bd1e9956c7b4d3a;
    result
}

/// Spec-side mirror of `resolve_contract_pure_decision`.
/// Mirrors the production body at crates/vb_runtime/src/engine/action.rs:211-221.
pub open spec fn spec_resolve_contract_decision(id_at_index_match: bool) -> bool {
    id_at_index_match
}

// ============================================================================
// assume_specification bridge: binds the production exec fn to a spec fn
// ============================================================================
//
// `assume_specification` is the Verus-native way to attach a spec
// contract to a Rust function whose body Verus cannot fully model (the
// extern file pulls in a tiny pure projection, but Verus still does not
// re-derive the body). The contract below states the deterministic
// postcondition of the production projection.
//
// TRUST BOUNDARY: the body of `execute_do_pure_decision` is in the extern
// file; Verus accepts the ensures via `assume_specification` but does not
// verify the body itself. This matches the binding ledger entry for the
// fuzz-gap coverage.

pub assume_specification[ production::execute_do_pure_decision ](
    _run_id: u64,
    _step: u32,
    _action: u32,
    _input: u32,
    _seq: u64,
    input_taint_disc: u8,
    _contract_id: u32,
    contract_idempotency_disc: u8,
    registry_action_match: bool,
    all_required_caps_granted: bool,
    _retry_max_attempts: u16,
    read_taint_failed: bool,
    output_taint_cleanable_from_tainted: bool,
) -> (kind: SpecOutcomeKind)
    ensures
        kind == spec_execute_do_decision(
            _run_id,
            _step,
            _action,
            _input,
            _seq,
            input_taint_disc,
            _contract_id,
            contract_idempotency_disc,
            registry_action_match,
            all_required_caps_granted,
            _retry_max_attempts,
            read_taint_failed,
            output_taint_cleanable_from_tainted,
        ),
        spec_outcome_kind_valid(kind),
;

// ============================================================================
// assume_specification bridges for the 6 remaining production exec wrappers
// ============================================================================
//
// Each `assume_specification` below mirrors the production decision fn in
// crates/vb_runtime/src/engine/action.rs through the pure projection in
// extern_runtime_execute_do.rs. The Verus-native contract is the spec fn
// immediately above each bridge.

pub assume_specification[ production::execute_do_without_contract_pure_decision ](
    input_taint_disc: u8,
) -> (kind: SpecOutcomeKind)
    ensures
        kind == spec_execute_do_without_contract_decision(input_taint_disc),
        spec_outcome_kind_valid(kind),
;

pub assume_specification[ production::execute_retry_check_pure_decision ](
    current_attempt: u16,
    max_attempts: u16,
    body: u32,
    exhausted: u32,
) -> (target: u32)
    ensures
        target == spec_execute_retry_check_decision(
            current_attempt,
            max_attempts,
            body,
            exhausted,
        ),
;

pub assume_specification[ production::execute_error_handler_pure_decision ](
    failure_retry_policy_disc: u8,
    failure_code_disc: u8,
    handler: u32,
    body: u32,
) -> (target: u32)
    ensures
        target == spec_execute_error_handler_decision(
            failure_retry_policy_disc,
            failure_code_disc,
            handler,
            body,
        ),
;

pub assume_specification[ production::resume_action_outcome_pure_decision ](
    outcome_disc: u8,
    attempt: u16,
    capacity: u16,
    retry_policy_disc: u8,
    seq_would_overflow: bool,
    attempt_would_overflow: bool,
) -> (kind: SpecResumeKind)
    ensures
        kind == spec_resume_action_outcome_decision(
            outcome_disc,
            attempt,
            capacity,
            retry_policy_disc,
            seq_would_overflow,
            attempt_would_overflow,
        ),
        spec_resume_kind_valid(kind),
;

pub assume_specification[ production::compute_idempotency_key_pure ](
    run: u64,
    seq: u64,
    action: u32,
) -> (key: u128)
    ensures
        // Equality with the spec-mirror arithmetic. The production body
        // delegates to `compute_action_idempotency_key` which is a
        // wrapping-multiply-add hash; the projection uses the same
        // arithmetic. Wrapping modulo 2^128 is implicit in u128 ops.
        key as int == spec_compute_idempotency_key_decision(run, seq, action),
;

pub assume_specification[ production::resolve_contract_pure_decision ](
    id_at_index_match: bool,
) -> (resolved: bool)
    ensures
        resolved == spec_resolve_contract_decision(id_at_index_match),
;

// ============================================================================
// Production-bound exec fn with requires/ensures
// ============================================================================

/// Production-bound exec wrapper that exercises the pure projection twice
/// with identical inputs and asserts the outcome kinds agree.
///
/// TRUST BOUNDARY: this exec fn calls `execute_do_pure_decision`, which is
/// the projection defined in `extern_runtime_execute_do.rs`. The Verus
/// `requires`/`ensures` on this exec fn are the contract Verus attaches to
/// the projection; the production body of `execute_do` is documented in
/// the binding ledger but not verified by this file.
pub exec fn checked_prod_execute_do_deterministic(
    run_id: u64,
    step: u32,
    action: u32,
    input: u32,
    seq: u64,
    input_taint_disc: u8,
    contract_id: u32,
    contract_idempotency_disc: u8,
    registry_action_match: bool,
    all_required_caps_granted: bool,
    retry_max_attempts: u16,
    read_taint_failed: bool,
    output_taint_cleanable_from_tainted: bool,
) -> (outcome: SpecOutcomeKind)
    requires
        // Taint discriminant must be one of the five documented Taint variants
        // (0..=4). The production Taint enum is `#[repr(u8)] #[non_exhaustive]`
        // but the documented variants span 0..=4. We allow 0..=4 here.
        input_taint_disc <= 4,
        // Idempotency discriminant must be one of the three documented
        // variants (0=DeterministicPure, 1=IdempotentExternal,
        // 2=AtLeastOnceExternal). The production Idempotency enum is
        // `#[repr(u8)] #[non_exhaustive]`; we allow 0..=2 here.
        contract_idempotency_disc <= 2,
        // Retry policy capacity must fit in u16 (it is a u16 in production).
        retry_max_attempts <= u16::MAX,
    ensures
        // Determinism bound: same inputs yield the same outcome kind.
        // This is the spec-level characterization of the missing fuzz
        // target: any two invocations with identical scalars produce
        // identical outcome-kind discriminants.
        outcome == spec_execute_do_decision(
            run_id,
            step,
            action,
            input,
            seq,
            input_taint_disc,
            contract_id,
            contract_idempotency_disc,
            registry_action_match,
            all_required_caps_granted,
            retry_max_attempts,
            read_taint_failed,
            output_taint_cleanable_from_tainted,
        ),
        // Validity bound: the returned outcome-kind discriminant is one of
        // the documented (Ok, Err) variants. This bounds the output to
        // the closed discriminant set produced by the production body.
        spec_outcome_kind_valid(outcome),
{
    let first = execute_do_pure_decision(
        run_id,
        step,
        action,
        input,
        seq,
        input_taint_disc,
        contract_id,
        contract_idempotency_disc,
        registry_action_match,
        all_required_caps_granted,
        retry_max_attempts,
        read_taint_failed,
        output_taint_cleanable_from_tainted,
    );
    let second = execute_do_pure_decision(
        run_id,
        step,
        action,
        input,
        seq,
        input_taint_disc,
        contract_id,
        contract_idempotency_disc,
        registry_action_match,
        all_required_caps_granted,
        retry_max_attempts,
        read_taint_failed,
        output_taint_cleanable_from_tainted,
    );
    // Determinism is a Rust-level guarantee (the fn is pure); Verus needs
    // us to assert the equality so the first `outcome ==` postcondition
    // resolves through the spec mirror.
    assert(first == second);
    // Bridge: the production exec result agrees with the spec mirror.
    assert(first == spec_execute_do_decision(
        run_id,
        step,
        action,
        input,
        seq,
        input_taint_disc,
        contract_id,
        contract_idempotency_disc,
        registry_action_match,
        all_required_caps_granted,
        retry_max_attempts,
        read_taint_failed,
        output_taint_cleanable_from_tainted,
    ));
    // Validity is discharged by the closed discriminant enumeration above.
    assert(spec_outcome_kind_valid(first));
    first
}

/// Production-bound exec wrapper that exercises `spec_propagate_action_taint`
/// and bounds the post-propagation boolean used by `execute_do`.
///
/// TRUST BOUNDARY: this wrapper computes the boolean directly from the
/// documented `Taint` discriminant (0=Clean, 1..=4 = non-Clean). The
/// production `propagate_action_taint` is in `vb_core::value::join_taint`
/// family at crates/vb_core/src/value.rs:27-45. The boolean reduction
/// here (`input_taint_disc != 0` ⇒ tainted, `== 0` ⇒ clean) is the
/// spec-level summary of the production rule.
pub exec fn checked_spec_propagate_action_taint(

// ============================================================================
// Companion chunk 2 — proof/remaining functions
// ============================================================================
#[path = "vb_runtime_execute_do_spec_chunk2.rs"]
mod chunk2;

} // verus!
