use super::RuntimeError;

pub(super) fn runtime_error_unit_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match runtime_error_unit_tag(left) {
        Some(tag) => runtime_error_unit_tag(right) == Some(tag),
        None => false,
    }
}

fn runtime_error_unit_tag(error: &RuntimeError) -> Option<u8> {
    match error {
        RuntimeError::QueueFull => Some(0),
        RuntimeError::RunNotFound => Some(1),
        RuntimeError::RunAlreadyExists => Some(2),
        RuntimeError::ShutdownInProgress => Some(3),
        RuntimeError::JournalPoisoned => Some(4),
        RuntimeError::UnsupportedAsyncStrictAck => Some(5),
        RuntimeError::FramePoolUnavailable => Some(6),
        RuntimeError::InvalidActionCompletion => Some(7),
        RuntimeError::InvalidTimerFire => Some(8),
        RuntimeError::UnsupportedFullRecoveryHydration => Some(9),
        RuntimeError::InvalidRecoveryHydration => Some(10),
        RuntimeError::ActiveRunCapacityZero => Some(11),
        RuntimeError::EncodeFailed => Some(12),
        RuntimeError::SecretResultNotAllowed => Some(13),
        RuntimeError::MigrateSelf => Some(15),
        RuntimeError::IntrospectionEpochExhausted => Some(16),
        _ => None,
    }
}
