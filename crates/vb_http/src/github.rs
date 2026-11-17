use crate::decode::{IntegerField, decode_integer_field};
use crate::{DecodeError, DecodeLimits};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScalarField {
    WorkflowCount,
}

pub(crate) fn decode_integer_body(
    body: &[u8],
    limits: DecodeLimits,
    field: ScalarField,
) -> Result<i64, DecodeError> {
    let ScalarField::WorkflowCount = field;
    match decode_integer_field(body, limits, "total_count")? {
        IntegerField::Missing => Err(DecodeError::MissingWorkflowCount),
        IntegerField::Duplicate => Err(DecodeError::DuplicateWorkflowCount),
        IntegerField::InvalidType => Err(DecodeError::InvalidWorkflowCount),
        IntegerField::Integer { negative: true, .. }
        | IntegerField::Integer {
            magnitude: None, ..
        } => Err(DecodeError::WorkflowCountOutOfRange),
        IntegerField::Integer {
            negative: false,
            magnitude: Some(value),
        } => i64::try_from(value).map_err(|_| DecodeError::WorkflowCountOutOfRange),
    }
}

#[cfg(test)]
#[path = "github_tests.rs"]
mod tests;
