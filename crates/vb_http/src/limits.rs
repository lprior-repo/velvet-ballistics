use crate::RequestRejection;
use std::time::Duration;

pub(crate) const MAX_BODY_BYTES: usize = 1_048_576;
const MAX_HEADER_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct HttpLimits {
    pub(crate) body_bytes: usize,
    pub(crate) header_bytes: usize,
    deadline: Duration,
}

impl HttpLimits {
    pub fn new(
        body_bytes: usize,
        header_bytes: usize,
        deadline: Duration,
    ) -> Result<Self, RequestRejection> {
        if body_bytes == 0
            || body_bytes > MAX_BODY_BYTES
            || header_bytes == 0
            || header_bytes > MAX_HEADER_BYTES
            || deadline.is_zero()
            || deadline > Duration::from_secs(60)
        {
            return Err(RequestRejection::Limits);
        }
        Ok(Self {
            body_bytes,
            header_bytes,
            deadline,
        })
    }

    pub const fn deadline(self) -> Duration {
        self.deadline
    }
}
