#[test]
fn deterministic_clean_stays_clean() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Clean);
    assert_eq!(result, Taint::Clean);
}

#[test]
fn deterministic_secret_stays_secret() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
    assert_eq!(result, Taint::Secret);
}

#[test]
fn deterministic_derived_stays_derived() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn idempotent_clean_stays_clean() {
    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::Clean);
    assert_eq!(result, Taint::Clean);
}

#[test]
fn at_least_once_secret_becomes_derived() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Secret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn at_least_once_derived_stays_derived() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn at_least_once_clean_stays_clean() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Clean);
    assert_eq!(result, Taint::Clean);
}

// -- ActionError exact variant assertions --

#[test]
fn action_error_unknown_action_exact_variant() -> Result<(), String> {
    let error = ActionError::UnknownAction {
        action: ActionId::new(42),
    };
    let ActionError::UnknownAction { action } = error else {
        return Err(String::from("expected UnknownAction variant"));
    };
    if action != ActionId::new(42) {
        return Err(String::from("unexpected action id"));
    }
    Ok(())
}

#[test]
fn action_error_invalid_ticket_exact_variant() {
    let error = ActionError::InvalidTicket;
    assert_eq!(error, ActionError::InvalidTicket);
}

#[test]
fn action_error_payload_too_large_exact_variant() -> Result<(), String> {
    let error = ActionError::PayloadTooLarge {
        max_bytes: 1024,
        actual_bytes: 2048,
    };
    let ActionError::PayloadTooLarge {
        max_bytes,
        actual_bytes,
    } = error
    else {
        return Err(String::from("expected PayloadTooLarge variant"));
    };
    if max_bytes != 1024 || actual_bytes != 2048 {
        return Err(String::from("unexpected payload size fields"));
    }
    Ok(())
}

#[test]
fn action_error_output_slot_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = ActionError::OutputSlotOutOfBounds {
        slot: 5,
        max_slots: 4,
    };
    let ActionError::OutputSlotOutOfBounds { slot, max_slots } = error else {
        return Err(String::from("expected OutputSlotOutOfBounds variant"));
    };
    if slot != 5 || max_slots != 4 {
        return Err(String::from("unexpected output slot bounds fields"));
    }
    Ok(())
}

#[test]
fn action_error_non_idempotent_replay_blocked_exact_variant() {
    let error = ActionError::NonIdempotentReplayBlocked;
    assert_eq!(error, ActionError::NonIdempotentReplayBlocked);
}

#[test]
fn action_error_completion_already_recorded_exact_variant() {
    let error = ActionError::CompletionAlreadyRecorded;
    assert_eq!(error, ActionError::CompletionAlreadyRecorded);
}

#[test]
fn action_error_queue_full_exact_variant() {
    let error = ActionError::QueueFull;
    assert_eq!(error, ActionError::QueueFull);
}

#[test]
fn action_error_encoding_failed_exact_variant() {
    let error = ActionError::EncodingFailed;
    assert_eq!(error, ActionError::EncodingFailed);
}

#[test]
fn action_error_dispatch_failed_exact_variant() {
    let error = ActionError::DispatchFailed;
    assert_eq!(error, ActionError::DispatchFailed);
}

#[test]
fn action_error_runtime_codes_cover_section_17_mappings() {
    assert_eq!(
        ActionError::UnknownAction {
            action: ActionId::new(9)
        }
        .runtime_code(),
        Some("REFERENCE_MISSING")
    );
    assert_eq!(
        ActionError::PayloadTooLarge {
            max_bytes: 1,
            actual_bytes: 2,
        }
        .runtime_code(),
        Some("PAYLOAD_TOO_LARGE")
    );
    assert_eq!(ActionError::QueueFull.runtime_code(), Some("QUEUE_FULL"));
    assert_eq!(
        ActionError::EncodingFailed.runtime_code(),
        Some("ACTION_FAILED")
    );
    assert_eq!(
        ActionError::DispatchFailed.runtime_code(),
        Some("ACTION_FAILED")
    );
}

#[test]
fn action_error_runtime_codes_are_unique() {
    let codes = [
        ActionError::REFERENCE_MISSING_RUNTIME_CODE,
        ActionError::ACTION_FAILED_RUNTIME_CODE,
        ActionError::PAYLOAD_TOO_LARGE_RUNTIME_CODE,
        ActionError::QUEUE_FULL_RUNTIME_CODE,
    ];
    assert_eq!(codes.len(), 4);
    assert_eq!(
        codes
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
}

#[test]
fn action_error_runtime_code_is_absent_without_section_17_equivalent() {
    assert_eq!(ActionError::InvalidTicket.runtime_code(), None);
    assert_eq!(ActionError::CompletionAlreadyRecorded.runtime_code(), None);
}

#[test]
fn encoded_action_input_len_computes_from_actual_payload_bytes() -> Result<(), String> {
    let action = ActionId::new(77);
    let contract = input_len_contract(action, 4)?;
    let payload = [0u8, 1, 2, 3];

    let checked = EncodedActionInputLen::from_encoded_payload(&payload, &contract)
        .map_err(|error| error.to_string())?;

    assert_eq!(checked.get(), 4);
    assert_eq!(checked.action(), action);
    Ok(())
}

#[test]
fn encoded_action_input_len_rejects_actual_payload_over_contract_limit() -> Result<(), String> {
    let contract = input_len_contract(ActionId::new(78), 3)?;
    let payload = [0u8, 1, 2, 3];

    assert_eq!(
        EncodedActionInputLen::from_encoded_payload(&payload, &contract),
        Err(ActionError::PayloadTooLarge {
            max_bytes: 3,
            actual_bytes: 4,
        })
    );
    Ok(())
}

#[test]
fn action_input_constructor_preserves_checked_len_and_rejects_ticket_mismatch() -> Result<(), String>
{
    let action = ActionId::new(79);
    let run = RunId::new(7);
    let step = StepIdx::new(2);
    let contract = input_len_contract(action, 8)?;
    let payload = [0u8, 1, 2, 3, 4];
    let ticket = input_len_ticket(run, step, action);
    let input = ActionInput::new(
        run,
        step,
        action,
        SlotIdx::ZERO,
        &payload,
        &contract,
        ticket,
    )
    .map_err(|error| error.to_string())?;

    assert_eq!(input.encoded_len(), 5);
    assert_eq!(input.encoded_input_len().get(), 5);

    let wrong_ticket = input_len_ticket(run, StepIdx::new(3), action);
    assert_eq!(
        ActionInput::new(
            run,
            step,
            action,
            SlotIdx::ZERO,
            &payload,
            &contract,
            wrong_ticket
        ),
        Err(ActionError::InvalidTicket)
    );
    Ok(())
}

#[test]
fn public_action_input_constructor_cannot_under_report_actual_payload_len() -> Result<(), String> {
    let action = ActionId::new(80);
    let run = RunId::new(8);
    let step = StepIdx::new(3);
    let contract = input_len_contract(action, 16)?;
    let payload = [0u8, 1, 2, 3, 4, 5, 6];
    let ticket = input_len_ticket(run, step, action);

    let input = ActionInput::new(
        run,
        step,
        action,
        SlotIdx::ZERO,
        &payload,
        &contract,
        ticket,
    )
    .map_err(|error| error.to_string())?;

    assert_eq!(input.encoded_len(), 7);
    Ok(())
}

#[test]
fn public_action_input_constructor_rejects_oversized_actual_payload() -> Result<(), String> {
    let action = ActionId::new(81);
    let contract = input_len_contract(action, 2)?;
    let payload = [0u8, 1, 2];
    let ticket = input_len_ticket(RunId::new(9), StepIdx::new(4), action);

    assert_eq!(
        ActionInput::new(
            RunId::new(9),
            StepIdx::new(4),
            action,
            SlotIdx::ZERO,
            &payload,
            &contract,
            ticket
        ),
        Err(ActionError::PayloadTooLarge {
            max_bytes: 2,
            actual_bytes: 3,
        })
    );
    Ok(())
}

// =========================================================================
// Phase 2 adversarial BDD tests -- action ABI security & taint vectors
// =========================================================================

#[test]
fn action_ticket_with_idempotency_key_zero_is_a_valid_ticket() {
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(5),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    };
    assert_eq!(ticket.idempotency_key, 0);
    assert_eq!(ticket.run, RunId::new(1));
}

#[test]
fn deterministic_pure_propagate_cannot_downgrade_secret_to_clean() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
    assert_eq!(result, Taint::Secret);
}

#[test]
fn deterministic_pure_propagate_cannot_downgrade_derived_to_clean() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn idempotent_external_propagate_cannot_downgrade_secret_to_clean() {
    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::Secret);
    assert_eq!(result, Taint::Secret);
}

#[test]
fn idempotent_external_propagate_cannot_downgrade_derived_to_clean() {
    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn at_least_once_secret_is_always_derived_never_secret() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Secret);
    assert_eq!(result, Taint::DerivedFromSecret);
    assert_ne!(result, Taint::Clean);
}

#[test]
fn at_least_once_derived_remains_derived_never_clean() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
    assert_ne!(result, Taint::Clean);
}

#[test]
fn action_ticket_from_different_run_is_not_equal() {
    let ticket_a = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(5),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 100,
        capacity: 1,
        fencing_token: 0,
    };
    let ticket_b = ActionTicket {
        run: RunId::new(2),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(5),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 100,
        capacity: 1,
        fencing_token: 0,
    };
    assert_ne!(ticket_a, ticket_b);
}
