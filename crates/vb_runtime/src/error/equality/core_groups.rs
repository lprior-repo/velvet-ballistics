use super::RuntimeError;

pub(super) fn runtime_error_gate_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    runtime_error_gate_state_eq(left, right) || runtime_error_gate_delegation_eq(left, right)
}

fn runtime_error_gate_state_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (RuntimeError::GateAlreadyOpen { gate: a }, RuntimeError::GateAlreadyOpen { gate: b }) => {
            a == b
        }
        (RuntimeError::GateNotOpen { gate: a }, RuntimeError::GateNotOpen { gate: b }) => a == b,
        (
            RuntimeError::GateJournalBudgetExceeded { gate: a },
            RuntimeError::GateJournalBudgetExceeded { gate: b },
        ) => a == b,
        (
            RuntimeError::GateEvaluationFailed { source: a },
            RuntimeError::GateEvaluationFailed { source: b },
        ) => a == b,
        (
            RuntimeError::GateRunNotSuspendable { run: a, state: sa },
            RuntimeError::GateRunNotSuspendable { run: b, state: sb },
        ) => a == b && sa == sb,
        (
            RuntimeError::GateOpenBlocksResume { run: a, gate: ga },
            RuntimeError::GateOpenBlocksResume { run: b, gate: gb },
        ) => a == b && ga == gb,
        _ => false,
    }
}

fn runtime_error_gate_delegation_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    gate_wrong_delegate_eq(left, right) || gate_run_mismatch_eq(left, right)
}

fn gate_wrong_delegate_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::GateWrongDelegate {
                gate: a,
                expected: ea,
                actual: aa,
            },
            RuntimeError::GateWrongDelegate {
                gate: b,
                expected: eb,
                actual: ab,
            },
        ) if a == b && ea == eb && aa == ab
    )
}

fn gate_run_mismatch_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::GateRunMismatch {
                gate: ga,
                expected: ea,
                actual: aa,
            },
            RuntimeError::GateRunMismatch {
                gate: gb,
                expected: eb,
                actual: ab,
            },
        ) if ga == gb && ea == eb && aa == ab
    )
}

pub(super) fn runtime_error_recovery_and_contract_eq(
    left: &RuntimeError,
    right: &RuntimeError,
) -> bool {
    recovery_error_eq(left, right)
        || runtime_input_kind_mismatch_eq(left, right)
        || contract_validation_eq(left, right)
}

fn recovery_error_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (RuntimeError::Recovery { error: a }, RuntimeError::Recovery { error: b }) if a == b
    )
}

fn runtime_input_kind_mismatch_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::RuntimeInputSlotKindMismatch {
                slot: a_slot,
                expected: a_exp,
                actual: a_act,
            },
            RuntimeError::RuntimeInputSlotKindMismatch {
                slot: b_slot,
                expected: b_exp,
                actual: b_act,
            },
        ) if a_slot == b_slot && a_exp == b_exp && a_act == b_act
    )
}

fn contract_validation_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::ContractValidationError {
                action: a_act,
                error: a_err,
            },
            RuntimeError::ContractValidationError {
                action: b_act,
                error: b_err,
            },
        ) if a_act == b_act && a_err == b_err
    )
}
