#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code))]

/// Machine-readable schema version emitted by the agent context.
pub(crate) const SCHEMA_VERSION: &str = "1";

/// Top-level kind identifier for the agent context payload.
pub(crate) const AGENT_CONTEXT_KIND: &str = "AgentContext";

/// Canonical CLI binary name.
pub(crate) const CLI_NAME: &str = "velvet-ballistics";

/// Language version identifier.
pub(crate) const LANGUAGE_VERSION: &str = "velvet-ballistics/v1";
