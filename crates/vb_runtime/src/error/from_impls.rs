use super::RuntimeError;

impl From<std::io::Error> for RuntimeError {
    fn from(_: std::io::Error) -> Self {
        RuntimeError::JournalPoisoned
    }
}

impl From<crate::shard::helpers::action::AttemptFenceError> for RuntimeError {
    fn from(e: crate::shard::helpers::action::AttemptFenceError) -> Self {
        match e {
            crate::shard::helpers::action::AttemptFenceError::StaleAttempt {
                incoming,
                current,
            } => RuntimeError::StaleAttempt { incoming, current },
            crate::shard::helpers::action::AttemptFenceError::AttemptBeyondMax { attempt, max } => {
                RuntimeError::AttemptBeyondMax { attempt, max }
            }
            crate::shard::helpers::action::AttemptFenceError::InvalidActionCompletion => {
                RuntimeError::InvalidActionCompletion
            }
        }
    }
}
