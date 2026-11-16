#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RequestRejection {
    #[error("HTTP endpoint is not permitted")]
    Endpoint,
    #[error("HTTP peer is not permitted")]
    Peer,
    #[error("HTTP resource limits are invalid")]
    Limits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AttemptFailure {
    #[error("HTTP transport failed")]
    Transport,
    #[error("HTTP attempt exceeded its deadline")]
    Timeout,
    #[error("HTTP response exceeded its header limit")]
    HeaderLimit,
    #[error("HTTP response exceeded its body limit")]
    BodyLimit,
    #[error("HTTP response encoding is unsupported")]
    Encoding,
    #[error("HTTP attempt could not reserve its bounded resources")]
    Resources,
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
