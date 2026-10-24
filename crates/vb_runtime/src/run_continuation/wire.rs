use super::payload::{VERSION, Wire};
use super::{ContinuationAuthority, RunContinuationError, RunContinuationV1};

impl RunContinuationV1 {
    /// Encodes complete validated component content; this is not a durable envelope.
    pub fn to_bytes(&self) -> Result<Vec<u8>, RunContinuationError> {
        Ok(postcard::to_allocvec(&self.data)?)
    }

    /// Decodes exactly one versioned payload and validates it before exposing it.
    ///
    /// The caller supplies already bounded component bytes. No journal tag,
    /// transport or durable-prefix provenance is established by this operation.
    pub fn from_bytes(
        bytes: &[u8],
        authority: &ContinuationAuthority,
    ) -> Result<Self, RunContinuationError> {
        let (version, _): (u16, _) = postcard::take_from_bytes(bytes)?;
        if version != VERSION {
            return Err(RunContinuationError::UnsupportedVersion { version });
        }
        let (data, rest): (Wire, _) = postcard::take_from_bytes(bytes)?;
        if !rest.is_empty() {
            return Err(RunContinuationError::TrailingBytes);
        }
        let result = Self { data };
        result.validate_against(authority)?;
        Ok(result)
    }
}
