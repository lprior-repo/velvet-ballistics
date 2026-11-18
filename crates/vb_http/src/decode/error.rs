#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("decode limits are invalid")]
    InvalidLimits,
    #[error("decode input exceeds its byte limit")]
    BodyLimit,
    #[error("decode nesting exceeds its depth limit")]
    DepthLimit,
    #[error("decode input exceeds its node limit")]
    NodeLimit,
    #[error("decoded string exceeds its byte limit")]
    StringLimit,
    #[error("input is not a complete supported JSON object")]
    InvalidJson,
    #[error("workflow count field is missing")]
    MissingWorkflowCount,
    #[error("workflow count field is duplicated")]
    DuplicateWorkflowCount,
    #[error("workflow count field has an invalid type")]
    InvalidWorkflowCount,
    #[error("workflow count is outside its nonnegative integer range")]
    WorkflowCountOutOfRange,
}
