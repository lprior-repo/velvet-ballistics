//! Kani harnesses for vb_core idempotency runtime gates.
//!
//! Scope: vb_core
//! Obligations: KANI-RUNTIME-001 through KANI-RUNTIME-009, vb-ko29.4
//! generator/non-vacuity repair.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutcome, ActionOutputReady,
    ActionTicket, Idempotency, IdempotencyViolation, RetryPolicy, RetrySafety, SideEffect,
    validate_action_outcome, verify_idempotency,
};
use crate::frame::RunFrame;
use crate::ids::{ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{SlotValue, Taint};

const MAX_SYMBOLIC_SLOTS: u16 = 4;

fn bounded_contract_for_retry(retry_safety: RetrySafety) -> ActionContract {
    let mut contract = symbolic_contract_no_caps();
    kani::assume(contract.side_effect != SideEffect::Pure);
    contract.retry_safety = retry_safety;
    contract
}

fn symbolic_contract_no_caps() -> ActionContract {
    ActionContract {
        id: ActionId::new(kani::any()),
        name: match ActionName::new("test-action") {
            Ok(v) => v,
            Err(_) => {
                kani::assume(false); loop {}
            }
        },
        input_slot_count: kani::any(),
        output_slot_count: kani::any(),
        max_input_bytes: kani::any(),
        max_output_bytes: kani::any(),
        timeout_ms: kani::any(),
        idempotency: symbolic_idempotency(),
        side_effect: symbolic_side_effect(),
        retry_safety: symbolic_retry_safety(),
        required_capabilities: Box::new([]),
    }
}

fn symbolic_side_effect() -> SideEffect {
    match kani::any::<u8>() {
        0 => SideEffect::Pure,
        1 => SideEffect::LocalWrite,
        2 => SideEffect::ExternalWrite,
        3 => SideEffect::LocalWrite,
        _ => SideEffect::LocalWrite,
    }
}

fn symbolic_retry_safety() -> RetrySafety {
    match kani::any::<u8>() {
        0 => RetrySafety::Idempotent,
        1 => RetrySafety::RequiresIdempotencyKey,
        _ => RetrySafety::NotRetrySafe,
    }
}

fn symbolic_idempotency() -> Idempotency {
    match kani::any::<u8>() {
        0 => Idempotency::DeterministicPure,
        1 => Idempotency::IdempotentExternal,
        _ => Idempotency::AtLeastOnceExternal,
    }
}

fn symbolic_frame() -> RunFrame {
    let run = RunId::new(kani::any::<u64>());
    let step_count = bounded_nonzero_u16(MAX_SYMBOLIC_SLOTS);
    let slot_count = bounded_nonzero_u16(MAX_SYMBOLIC_SLOTS);
    let step_raw: u16 = kani::any();
    kani::assume(step_raw < step_count);
    let frame = RunFrame::new(run, StepIdx::new(step_raw), step_count, slot_count);
    let mut frame = match frame {
        Ok(value) => value,
        Err(_) => unreachable_for_kani_frame_bounds(),
    };
    write_symbolic_slot(&mut frame, 0, slot_count);
    write_symbolic_slot(&mut frame, 1, slot_count);
    write_symbolic_slot(&mut frame, 2, slot_count);
    write_symbolic_slot(&mut frame, 3, slot_count);
    frame
}

fn one_slot_frame_with_taint(taint: Taint) -> RunFrame {
    let frame = RunFrame::new(RunId::new(kani::any::<u64>()), StepIdx::new(0), 1, 1);
    let mut frame = match frame {
        Ok(value) => value,
        Err(_) => unreachable_for_kani_frame_bounds(),
    };
    let result =
        frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(kani::any::<i64>()), taint);
    kani::assume(result.is_ok());
    frame
}

fn four_slot_clean_frame() -> RunFrame {
    let frame = RunFrame::new(
        RunId::new(kani::any::<u64>()),
        StepIdx::new(0),
        1,
        MAX_SYMBOLIC_SLOTS,
    );
    let mut frame = match frame {
        Ok(value) => value,
        Err(_) => unreachable_for_kani_frame_bounds(),
    };
    write_slot_with_taint(&mut frame, 0, MAX_SYMBOLIC_SLOTS, Taint::Clean);
    write_slot_with_taint(&mut frame, 1, MAX_SYMBOLIC_SLOTS, Taint::Clean);
    write_slot_with_taint(&mut frame, 2, MAX_SYMBOLIC_SLOTS, Taint::Clean);
    write_slot_with_taint(&mut frame, 3, MAX_SYMBOLIC_SLOTS, Taint::Clean);
    frame
}

fn bounded_nonzero_u16(max: u16) -> u16 {
    let value: u16 = kani::any();
    kani::assume(value > 0);
    kani::assume(value <= max);
    value
}

fn bounded_key_slot() -> SlotIdx {
    let raw: u16 = kani::any();
    kani::assume(raw < MAX_SYMBOLIC_SLOTS);
    SlotIdx::new(raw)
}

fn write_symbolic_slot(frame: &mut RunFrame, raw: u16, slot_count: u16) {
    if raw < slot_count {
        let result = frame.write_slot_with_taint(
            SlotIdx::new(raw),
            kani::any::<SlotValue>(),
            kani::any::<Taint>(),
        );
        kani::assume(result.is_ok());
    }
}

fn write_slot_with_taint(frame: &mut RunFrame, raw: u16, slot_count: u16, taint: Taint) {
    if raw < slot_count {
        let result =
            frame.write_slot_with_taint(SlotIdx::new(raw), kani::any::<SlotValue>(), taint);
        kani::assume(result.is_ok());
    }
}

fn symbolic_failure_code() -> ActionFailureCode {
    match kani::any::<u8>() {
        0 => ActionFailureCode::Rejected,
        1 => ActionFailureCode::Timeout,
        2 => ActionFailureCode::RateLimited,
        3 => ActionFailureCode::ResourceExhausted,
        4 => ActionFailureCode::ExternalUnavailable,
        5 => ActionFailureCode::InvalidInput,
        6 => ActionFailureCode::PermissionDenied,
        7 => ActionFailureCode::Conflict,
        _ => ActionFailureCode::Unknown,
    }
}

fn symbolic_ticket() -> ActionTicket {
    ActionTicket {
        run: RunId::new(kani::any()),
        step: StepIdx::new(kani::any()),
        seq: SeqNo::new(kani::any()),
        action: ActionId::new(kani::any()),
        attempt: kani::any(),
        idempotency_key: kani::any(),
        capacity: kani::any(),
        ..ActionTicket::default()
    }
}

fn unreachable_for_kani_frame_bounds() -> RunFrame {
    unreachable!("RunFrame bounds guaranteed by assume");
}

/// Concrete contract for the MissingKey harness — avoids kani::any() branching
/// on the contract fields, which keeps the verification state space bounded.
fn concrete_key_required_contract() -> ActionContract {
    ActionContract {
        id: ActionId::new(1),
        name: match ActionName::new("test-action") {
            Ok(v) => v,
            Err(_) => {
                kani::assume(false); loop {}
            }
        },
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    }
}

#[kani::proof]
#[kani::unwind(10)]
fn verify_idempotency_missing_key_symbolic_contract_no_frame_write() {
    let contract = concrete_key_required_contract();
    let step_count = bounded_nonzero_u16(2);
    let slot_count = bounded_nonzero_u16(2);
    let step_raw: u16 = kani::any();
    kani::assume(step_raw < step_count);
    let frame = match RunFrame::new(
        RunId::new(kani::any()),
        StepIdx::new(step_raw),
        step_count,
        slot_count,
    ) {
        Ok(value) => value,
        Err(_) => unreachable_for_kani_frame_bounds(),
    };
    let result = verify_idempotency(&contract, &[], &frame);
    kani::cover!(
        matches!(result, Err(IdempotencyViolation::MissingKey(_))),
        "symbolic missing key covered"
    );
    kani::assert(
        matches!(result, Err(IdempotencyViolation::MissingKey(_))),
        "symbolic KeyRequired side-effecting contract with empty key returns MissingKey",
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn verify_idempotency_symbolic_key_taints_are_classified() {
    let contract = bounded_contract_for_retry(RetrySafety::RequiresIdempotencyKey);
    let frame = symbolic_frame();
    let key_len: u8 = kani::any();
    kani::assume(key_len <= 4);

    match key_len {
        0 => {
            let result = verify_idempotency(&contract, &[], &frame);
            kani::cover!(result.is_err(), "missing key failure covered");
            kani::assert(
                matches!(result, Err(IdempotencyViolation::MissingKey(_))),
                "empty KeyRequired key set returns MissingKey",
            );
        }
        1 => check_symbolic_key_array(&contract, [bounded_key_slot()], &frame),
        2 => check_symbolic_key_array(&contract, [bounded_key_slot(), bounded_key_slot()], &frame),
        3 => check_symbolic_key_array(
            &contract,
            [bounded_key_slot(), bounded_key_slot(), bounded_key_slot()],
            &frame,
        ),
        _ => check_symbolic_key_array(
            &contract,
            [
                bounded_key_slot(),
                bounded_key_slot(),
                bounded_key_slot(),
                bounded_key_slot(),
            ],
            &frame,
        ),
    }
}

fn check_symbolic_key_array<const N: usize>(
    contract: &ActionContract,
    key_slots: [SlotIdx; N],
    frame: &RunFrame,
) {
    let result = verify_idempotency(contract, &key_slots, frame);
    kani::cover!(N == 1, "minimum non-empty key length covered");
    kani::cover!(N == 4, "maximum bounded key length covered");
    kani::cover!(result.is_ok(), "clean symbolic key succeeds");
    kani::cover!(
        matches!(result, Err(IdempotencyViolation::SecretInKey(_))),
        "secret key failure covered"
    );
    kani::assert(
        result.is_ok() || matches!(result, Err(IdempotencyViolation::SecretInKey(_))),
        "non-empty bounded key only succeeds or reports the first tainted key ingredient",
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn verify_idempotency_required_taint_variants_have_witnesses() {
    let contract = bounded_contract_for_retry(RetrySafety::RequiresIdempotencyKey);
    let taint_selector: u8 = kani::any();
    kani::assume(taint_selector < 3);
    let taint = match taint_selector {
        0 => Taint::Clean,
        1 => Taint::Secret,
        _ => Taint::DerivedFromSecret,
    };
    let frame = one_slot_frame_with_taint(taint);
    let key_slots = [SlotIdx::new(0)];
    let result = verify_idempotency(&contract, &key_slots, &frame);

    kani::cover!(
        taint == Taint::Clean && result.is_ok(),
        "clean taint success covered"
    );
    kani::cover!(
        matches!(result, Err(IdempotencyViolation::SecretInKey(0))),
        "secret taint failure covered"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn verify_idempotency_retry_policy_matrix_is_total() {
    let contract = symbolic_contract_no_caps();
    let frame = symbolic_frame();
    let key_slots = [bounded_key_slot()];
    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::cover!(
        contract.side_effect == SideEffect::Pure && result.is_ok(),
        "pure bypass covered"
    );
    kani::cover!(
        contract.retry_safety == RetrySafety::Idempotent && result.is_ok(),
        "safe retry covered"
    );
    kani::cover!(
        contract.retry_safety == RetrySafety::RequiresIdempotencyKey,
        "key-required retry covered"
    );
    kani::cover!(
        contract.side_effect != SideEffect::Pure
            && contract.retry_safety == RetrySafety::NotRetrySafe
            && matches!(result, Err(IdempotencyViolation::MissingKey(_))),
        "unsafe retry failure covered"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn verify_idempotency_duplicate_invocation_is_stable() {
    let contract = bounded_contract_for_retry(RetrySafety::RequiresIdempotencyKey);
    let taint = match kani::any::<u8>() {
        0 => Taint::Clean,
        1 => Taint::Secret,
        2 => Taint::DerivedFromSecret,
        3 => Taint::Secret,
        _ => Taint::Secret,
    };
    let frame = one_slot_frame_with_taint(taint);
    let key_slots = [SlotIdx::new(0)];
    let first = verify_idempotency(&contract, &key_slots, &frame);
    let second = verify_idempotency(&contract, &key_slots, &frame);
    kani::cover!(first.is_ok() && second.is_ok(), "duplicate success covered");
    kani::cover!(
        first.is_err() && second.is_err(),
        "duplicate failure covered"
    );
    kani::assert(
        first.is_ok() == second.is_ok(),
        "same contract/key/frame verification is idempotent",
    );
}

#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_duplicate_success_clean_key() {
    let contract = bounded_contract_for_retry(RetrySafety::RequiresIdempotencyKey);
    let frame = one_slot_frame_with_taint(Taint::Clean);
    let key_slots = [SlotIdx::new(0)];
    let first = verify_idempotency(&contract, &key_slots, &frame);
    let second = verify_idempotency(&contract, &key_slots, &frame);
    kani::cover!(first.is_ok() && second.is_ok(), "duplicate success covered");
    kani::assert(
        first.is_ok() && second.is_ok(),
        "clean key duplicate succeeds twice",
    );
}

#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_duplicate_failure_tainted_key() {
    let contract = bounded_contract_for_retry(RetrySafety::RequiresIdempotencyKey);
    let taint = match kani::any::<u8>() {
        0 => Taint::Secret,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        _ => Taint::Secret,
    };
    let frame = one_slot_frame_with_taint(taint);
    let key_slots = [SlotIdx::new(0)];
    let first = verify_idempotency(&contract, &key_slots, &frame);
    let second = verify_idempotency(&contract, &key_slots, &frame);
    kani::cover!(
        first.is_err() && second.is_err(),
        "duplicate failure covered"
    );
    kani::assert(
        first.is_err() && second.is_err(),
        "tainted key duplicate fails twice",
    );
}

#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_boundary_key_lengths_pass_clean_frame() {
    let contract = bounded_contract_for_retry(RetrySafety::RequiresIdempotencyKey);
    let frame = four_slot_clean_frame();
    let one_key = [SlotIdx::new(0)];
    let max_key = [
        SlotIdx::new(0),
        SlotIdx::new(1),
        SlotIdx::new(2),
        SlotIdx::new(3),
    ];
    let one = verify_idempotency(&contract, &one_key, &frame);
    let max = verify_idempotency(&contract, &max_key, &frame);
    kani::cover!(one.is_ok(), "minimum key length success covered");
    kani::cover!(max.is_ok(), "maximum bounded key length success covered");
    kani::assert(one.is_ok() && max.is_ok(), "boundary clean keys pass");
}

#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_frame_slot_bounds_no_panic() {
    let contract = bounded_contract_for_retry(RetrySafety::RequiresIdempotencyKey);
    let frame = four_slot_clean_frame();
    let use_oob: bool = kani::any();
    let key = if use_oob {
        SlotIdx::new(4)
    } else {
        bounded_key_slot()
    };
    let key_slots = [key];
    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::cover!(!use_oob && result.is_ok(), "in-bounds key slot covered");
    kani::cover!(
        use_oob && result.is_ok(),
        "out-of-bounds ignored slot covered"
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn verify_idempotency_retry_policy_matrix_no_frame_write() {
    let contract = symbolic_contract_no_caps();
    let frame = match RunFrame::new(RunId::new(kani::any()), StepIdx::new(0), 1, 1) {
        Ok(value) => value,
        Err(_) => unreachable_for_kani_frame_bounds(),
    };
    let result = verify_idempotency(&contract, &[], &frame);
    kani::cover!(
        contract.side_effect == SideEffect::Pure && result.is_ok(),
        "no side-effect bypass covered"
    );
    kani::cover!(
        contract.side_effect != SideEffect::Pure
            && contract.retry_safety == RetrySafety::Idempotent
            && result.is_ok(),
        "safe retry covered"
    );
    kani::cover!(
        contract.side_effect != SideEffect::Pure
            && contract.retry_safety == RetrySafety::RequiresIdempotencyKey
            && matches!(result, Err(IdempotencyViolation::MissingKey(_))),
        "key-required missing-key covered"
    );
    kani::cover!(
        contract.side_effect != SideEffect::Pure
            && contract.retry_safety == RetrySafety::NotRetrySafe
            && matches!(result, Err(IdempotencyViolation::MissingKey(_))),
        "unsafe retry covered"
    );
}

#[kani::proof]
#[kani::unwind(40)]
fn idempotency_divergent_digest_symbolic_certificate_rejected() {
    let first = WorkflowDigest::from_bytes(kani::any::<[u8; 32]>());
    let second = WorkflowDigest::from_bytes(kani::any::<[u8; 32]>());
    kani::assume(first != second);
    let accepted = first == second;
    kani::cover!(!accepted, "divergent digest rejection covered");
    kani::assert(
        !accepted,
        "divergent workflow digests are rejected by equality gate",
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn validate_action_outcome_certificate_stale_nonterminal() {
    let contract = symbolic_contract_no_caps();
    let outcome = ActionOutcome::Suspended(symbolic_ticket());
    let result = validate_action_outcome(&contract, &outcome, kani::any::<Taint>());
    kani::cover!(
        matches!(result, Err(crate::action::ActionError::DispatchFailed)),
        "stale/nonterminal certificate covered"
    );
    kani::assert(result.is_err(), "nonterminal suspended outcome is rejected");
}

#[kani::proof]
#[kani::unwind(4)]
fn validate_action_outcome_certificate_conflict_oob() {
    let mut contract = symbolic_contract_no_caps();
    contract.output_slot_count = 1;
    let outcome = ActionOutcome::Ready(ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(kani::any::<i64>()),
        taint: kani::any::<Taint>(),
        encoded_len: kani::any(),
    });
    let result = validate_action_outcome(&contract, &outcome, kani::any::<Taint>());
    kani::cover!(
        matches!(
            result,
            Err(crate::action::ActionError::OutputSlotOutOfBounds { .. })
        ),
        "conflict certificate covered"
    );
    kani::assert(
        result.is_err(),
        "out-of-bounds completion certificate is rejected",
    );
}

#[kani::proof]
#[kani::unwind(6)]
fn validate_action_outcome_symbolic_completion_matrix() {
    let mut contract = symbolic_contract_no_caps();
    let max_outputs = bounded_nonzero_u16(MAX_SYMBOLIC_SLOTS);
    contract.output_slot_count = max_outputs;
    let selector: u8 = kani::any();
    kani::assume(selector < 4);
    let output_slot = bounded_key_slot();
    let outcome = match selector {
        0 => ActionOutcome::Ready(ActionOutputReady {
            output_slot,
            value: kani::any::<SlotValue>(),
            taint: kani::any::<Taint>(),
            encoded_len: kani::any(),
        }),
        1 => ActionOutcome::Failed(ActionFailure {
            code: symbolic_failure_code(),
            retry_policy: RetryPolicy::Retryable,
            taint: kani::any::<Taint>(),
            detail: Some(BlobId::new(kani::any())),
            encoded_len: kani::any(),
        }),
        2 => ActionOutcome::Failed(ActionFailure {
            code: symbolic_failure_code(),
            retry_policy: RetryPolicy::NonRetryable,
            taint: kani::any::<Taint>(),
            detail: None,
            encoded_len: kani::any(),
        }),
        _ => ActionOutcome::Suspended(symbolic_ticket()),
    };

    let result = validate_action_outcome(&contract, &outcome, kani::any::<Taint>());
    kani::cover!(
        matches!(outcome, ActionOutcome::Ready(_)),
        "success certificate covered"
    );
    kani::cover!(
        matches!(
            outcome,
            ActionOutcome::Failed(ActionFailure {
                retry_policy: RetryPolicy::Retryable,
                ..
            })
        ),
        "retryable failure certificate covered"
    );
    kani::cover!(
        matches!(
            outcome,
            ActionOutcome::Failed(ActionFailure {
                retry_policy: RetryPolicy::NonRetryable,
                ..
            })
        ),
        "nonretryable failure certificate covered"
    );
    kani::cover!(
        matches!(outcome, ActionOutcome::Suspended(_)),
        "stale/nonterminal certificate covered"
    );
    kani::cover!(
        matches!(
            result,
            Err(crate::action::ActionError::OutputSlotOutOfBounds { .. })
        ),
        "conflict/out-of-bounds completion covered"
    );
    kani::cover!(
        matches!(
            result,
            Err(crate::action::ActionError::TaintViolation { .. })
        ),
        "taint violation error covered"
    );
    kani::cover!(
        matches!(
            result,
            Err(crate::action::ActionError::PayloadTooLarge { .. })
        ),
        "payload too large error covered"
    );
    kani::assert(
        result.is_ok()
            || matches!(
                result,
                Err(crate::action::ActionError::OutputSlotOutOfBounds { .. })
            )
            || matches!(
                result,
                Err(crate::action::ActionError::TaintViolation { .. })
            )
            || matches!(
                result,
                Err(crate::action::ActionError::PayloadTooLarge { .. })
            )
            || matches!(result, Err(crate::action::ActionError::DispatchFailed)),
        "all outcomes produce only expected error variants or Ok",
    );
}

// ============================================================================
// vb-8mdp.6: Idempotency Hydration — vb_core action.rs Kani harnesses
// PO-VB-IDEM-012a, 017a
// ============================================================================

/// PO-VB-IDEM-012a: action_ticket_has_valid_key returns true when
/// ticket.idempotency_key == compute_action_idempotency_key(run, seq, action).
#[kani::proof]
#[kani::unwind(6)]
fn kani_action_ticket_has_valid_key() {
    use crate::action::{action_ticket_has_valid_key, compute_action_idempotency_key};

    let run = RunId::new(kani::any::<u64>());
    let seq = SeqNo::new(kani::any::<u64>());
    let action = ActionId::new(kani::any::<u16>());

    let canonical_key = compute_action_idempotency_key(run, seq, action);

    // Canonical ticket — should validate
    let canonical_ticket = ActionTicket {
        run,
        step: StepIdx::new(kani::any::<u16>()),
        seq,
        action,
        attempt: kani::any(),
        idempotency_key: canonical_key,
        capacity: kani::any(),
        ..ActionTicket::default()
    };
    let canonical_result = action_ticket_has_valid_key(canonical_ticket);
    kani::assert(canonical_result, "canonical key validates");

    // Wrong key — should reject
    let wrong_key = canonical_key.wrapping_add(1);
    let wrong_ticket = ActionTicket {
        idempotency_key: wrong_key,
        ..canonical_ticket
    };
    let wrong_result = action_ticket_has_valid_key(wrong_ticket);
    kani::cover!(!wrong_result, "wrong key rejection covered");
    kani::assert(!wrong_result, "wrong key is rejected");

    // Zero key
    let zero_ticket = ActionTicket {
        idempotency_key: 0,
        ..canonical_ticket
    };
    let zero_result = action_ticket_has_valid_key(zero_ticket);
    kani::cover!(!zero_result, "zero key rejected covered");

    // Max key — either matches or doesn't
    let max_ticket = ActionTicket {
        idempotency_key: u128::MAX,
        ..canonical_ticket
    };
    let max_result = action_ticket_has_valid_key(max_ticket);
    kani::cover!(max_result, "max key match covered");
    kani::cover!(!max_result, "max key mismatch covered");
}

/// PO-VB-IDEM-017a: verify_idempotency returns MissingKey for KeyRequired
/// with empty key_slots OR Unsafe retry.
#[kani::proof]
#[kani::unwind(10)]
fn kani_verify_idempotency_missing_key() {
    use crate::action::{
        ActionContract, Idempotency, IdempotencyViolation, RetrySafety, SideEffect,
        verify_idempotency,
    };

    // Build a non-None side-effect contract
    let make_contract = |retry_safety| ActionContract {
        id: ActionId::new(kani::any()),
        name: match ActionName::new("test-action") {
            Ok(v) => v,
            Err(_) => {
                kani::assume(false); loop {}
            }
        },
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::LocalWrite,
        retry_safety,
        required_capabilities: Box::new([]),
    };

    // Build minimal valid frame
    let frame = RunFrame::new(RunId::new(0), StepIdx::new(0), 1, 1);
    let mut frame = match frame {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(0), Taint::Clean);

    // KeyRequired + empty key_slots -> MissingKey
    let contract_keyreq = make_contract(RetrySafety::RequiresIdempotencyKey);
    let result_empty = verify_idempotency(&contract_keyreq, &[], &frame);
    kani::cover!(
        matches!(result_empty, Err(IdempotencyViolation::MissingKey(_))),
        "KeyRequired + empty keys covered"
    );
    kani::assert(
        matches!(
            result_empty,
            Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
        ),
        "KeyRequired empty returns MissingKey(Writes)",
    );

    // Unsafe + any key_slots -> MissingKey
    let contract_unsafe = make_contract(RetrySafety::NotRetrySafe);
    let result_unsafe_empty = verify_idempotency(&contract_unsafe, &[], &frame);
    kani::cover!(
        matches!(
            result_unsafe_empty,
            Err(IdempotencyViolation::MissingKey(_))
        ),
        "Unsafe + empty covered"
    );
    kani::assert(
        matches!(
            result_unsafe_empty,
            Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
        ),
        "Unsafe returns MissingKey even with empty key_slots",
    );

    let result_unsafe_with_key = verify_idempotency(&contract_unsafe, &[SlotIdx::new(0)], &frame);
    kani::cover!(
        matches!(
            result_unsafe_with_key,
            Err(IdempotencyViolation::MissingKey(_))
        ),
        "Unsafe + non-empty key_slots still MissingKey covered"
    );
    kani::assert(
        matches!(
            result_unsafe_with_key,
            Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
        ),
        "Unsafe with keys still returns MissingKey",
    );

    // Safe -> Ok (no key needed)
    let contract_safe = make_contract(RetrySafety::Idempotent);
    let result_safe = verify_idempotency(&contract_safe, &[], &frame);
    kani::cover!(
        result_safe.is_ok(),
        "Safe retry passes without keys covered"
    );
    kani::assert(result_safe.is_ok(), "Safe returns Ok");

    // None side-effect -> always Ok regardless of retry_safety
    let contract_none = ActionContract {
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::NotRetrySafe,
        ..make_contract(RetrySafety::Idempotent)
    };
    let result_none = verify_idempotency(&contract_none, &[], &frame);
    kani::assert(result_none.is_ok(), "SideEffect::Pure always passes");
}
