use super::RuntimeError;

pub(super) fn runtime_error_core_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    runtime_error_capacity_eq(left, right)
        || runtime_error_operation_eq(left, right)
        || runtime_error_persistence_eq(left, right)
        || runtime_error_attempt_eq(left, right)
        || runtime_error_payload_eq(left, right)
        || runtime_error_action_output_eq(left, right)
        || runtime_error_action_taint_eq(left, right)
        || runtime_error_journal_eq(left, right)
        || runtime_error_shard_eq(left, right)
        || runtime_error_recovery_and_contract_eq(left, right)
}

fn runtime_error_capacity_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::ActiveRunCapacityExceeded { capacity: a },
            RuntimeError::ActiveRunCapacityExceeded { capacity: b },
        ) => a == b,
        (RuntimeError::JournalFull { capacity: a }, RuntimeError::JournalFull { capacity: b }) => {
            a == b
        }
        (
            RuntimeError::CommandQueueCapacityExceeded {
                capacity: a,
                max: b,
            },
            RuntimeError::CommandQueueCapacityExceeded {
                capacity: c,
                max: d,
            },
        ) => a == c && b == d,
        _ => false,
    }
}

fn runtime_error_operation_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::UnsupportedOperation { operation: a },
            RuntimeError::UnsupportedOperation { operation: b },
        ) => a == b,
        (
            RuntimeError::RecoveryCannotResume { reason: a },
            RuntimeError::RecoveryCannotResume { reason: b },
        ) => a == b,
        (RuntimeError::Core { source: a }, RuntimeError::Core { source: b }) => a == b,
        (
            RuntimeError::EngineDriveFailed { run: a, source: sa },
            RuntimeError::EngineDriveFailed { run: b, source: sb },
        ) => a == b && sa.diagnostic_code() == sb.diagnostic_code(),
        _ => false,
    }
}

fn runtime_error_persistence_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::StorageJournalAppend { source: a },
            RuntimeError::StorageJournalAppend { source: b },
        ) => a.diagnostic_code() == b.diagnostic_code(),
        (
            RuntimeError::AdmissionHeaderPersistenceFailed { source: a },
            RuntimeError::AdmissionHeaderPersistenceFailed { source: b },
        ) => a.diagnostic_code() == b.diagnostic_code(),
        (
            RuntimeError::RollbackFailed {
                operation: operation_a,
                primary: primary_a,
                rollback: rollback_a,
            },
            RuntimeError::RollbackFailed {
                operation: operation_b,
                primary: primary_b,
                rollback: rollback_b,
            },
        ) => operation_a == operation_b && primary_a == primary_b && rollback_a == rollback_b,
        _ => false,
    }
}

fn runtime_error_attempt_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::StaleAttempt {
                incoming: a,
                current: b,
            },
            RuntimeError::StaleAttempt {
                incoming: c,
                current: d,
            },
        ) => a == c && b == d,
        (
            RuntimeError::AttemptBeyondMax { attempt: a, max: b },
            RuntimeError::AttemptBeyondMax { attempt: c, max: d },
        ) => a == c && b == d,
        _ => false,
    }
}

fn runtime_error_payload_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::IpcPayloadSizeExceeded { size: a, max: b },
            RuntimeError::IpcPayloadSizeExceeded { size: c, max: d },
        ) => a == c && b == d,
        (
            RuntimeError::ActionOutputLengthMismatch {
                declared: a,
                actual: b,
            },
            RuntimeError::ActionOutputLengthMismatch {
                declared: c,
                actual: d,
            },
        ) => a == c && b == d,
        _ => false,
    }
}

fn runtime_error_action_output_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::ActionOutputTooLarge { size: a, max: b },
            RuntimeError::ActionOutputTooLarge { size: c, max: d },
        ) => a == c && b == d,
        (
            RuntimeError::ActionOutputBlobTooLarge { size: a, max: b },
            RuntimeError::ActionOutputBlobTooLarge { size: c, max: d },
        ) => a == c && b == d,
        _ => false,
    }
}

fn runtime_error_action_taint_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::ActionTaintDowngrade {
                required: a,
                supplied: b,
            },
            RuntimeError::ActionTaintDowngrade {
                required: c,
                supplied: d,
            },
        ) => a == c && b == d,
        _ => false,
    }
}

fn runtime_error_journal_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::UnsupportedRuntimeJournalEventMapping { event_kind: a },
            RuntimeError::UnsupportedRuntimeJournalEventMapping { event_kind: b },
        ) => a == b,
        (
            RuntimeError::RuntimeJournalTimestampOutOfRange {
                event_kind: a,
                timestamp: b,
            },
            RuntimeError::RuntimeJournalTimestampOutOfRange {
                event_kind: c,
                timestamp: d,
            },
        ) => a == c && b == d,
        _ => false,
    }
}

fn runtime_error_shard_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (RuntimeError::ShardNotFound { shard: a }, RuntimeError::ShardNotFound { shard: b })
            if a == b
    )
}

fn runtime_error_recovery_and_contract_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (RuntimeError::Recovery { error: a }, RuntimeError::Recovery { error: b }) => a == b,
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
        ) => a_slot == b_slot && a_exp == b_exp && a_act == b_act,
        (
            RuntimeError::ContractValidationError {
                action: a_act,
                error: a_err,
            },
            RuntimeError::ContractValidationError {
                action: b_act,
                error: b_err,
            },
        ) => a_act == b_act && a_err == b_err,
        _ => false,
    }
}
