use crate::github::{ScalarField, decode_integer_body};
use crate::{DecodeError, DecodeLimits};

/// Exact nonnegative workflow count, distinct from a repository identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowCount(i64);

impl WorkflowCount {
    /// Runtime scalar representation; external provenance remains unchanged.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for WorkflowCount {
    type Error = DecodeError;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(Self(value))
        } else {
            Err(DecodeError::WorkflowCountOutOfRange)
        }
    }
}

/// Decode total_count from a complete workflows response under explicit bounds.
/// This projects the count; it does not return or validate workflow definitions.
pub fn decode_workflow_count_body(
    body: &[u8],
    limits: DecodeLimits,
) -> Result<WorkflowCount, DecodeError> {
    decode_integer_body(body, limits, ScalarField::WorkflowCount).and_then(WorkflowCount::try_from)
}
