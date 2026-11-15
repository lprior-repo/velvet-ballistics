use super::DecodeError;

pub(super) const MAX_INPUT_BYTES: usize = 1_048_576;
pub(super) const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct DecodeLimits {
    pub(super) body_bytes: usize,
    pub(super) depth: usize,
    pub(super) nodes: usize,
    pub(super) string_bytes: usize,
}

impl DecodeLimits {
    /// Budgets count containers/values and object keys as nodes; depth counts containers.
    /// String limits count decoded UTF-8 bytes separately for each key/value string.
    pub fn new(
        body_bytes: usize,
        depth: usize,
        nodes: usize,
        string_bytes: usize,
    ) -> Result<Self, DecodeError> {
        if body_bytes == 0
            || body_bytes > MAX_INPUT_BYTES
            || depth == 0
            || depth > MAX_DEPTH
            || nodes == 0
            || nodes > MAX_INPUT_BYTES
            || string_bytes == 0
            || string_bytes > MAX_INPUT_BYTES
        {
            return Err(DecodeError::InvalidLimits);
        }
        Ok(Self {
            body_bytes,
            depth,
            nodes,
            string_bytes,
        })
    }
}
