#![forbid(unsafe_code)]
//! Terminal action outcome captured by durable completion envelopes.

/// Terminal action outcome captured by durable completion envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum DurableActionOutcome {
    /// Action completed successfully and wrote an output slot.
    Ready = 1,
}
