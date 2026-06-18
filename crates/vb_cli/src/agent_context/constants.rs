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

/// Active policy-blocker entries emitted by `policy::known_blockers`.
pub(crate) const POLICY_BLOCKER_COUNT: usize = 8;

/// Active resource-blocker entries emitted by `policy::known_blockers`.
pub(crate) const RESOURCE_BLOCKER_COUNT: usize = 3;

/// Active capability-blocker entries emitted by `policy::known_blockers`.
pub(crate) const CAPABILITY_BLOCKER_COUNT: usize = 3;

/// Defined CLI exit codes emitted by `policy::exit_codes` (`0` through `8`).
pub(crate) const EXIT_CODE_COUNT: usize = 9;

/// Enum groups emitted by `policy::enums`.
pub(crate) const ENUM_COUNT: usize = 4;

/// Command definitions emitted by `commands::commands`.
pub(crate) const COMMAND_COUNT: usize = 30;

/// Vocabulary array groups covered by the Kani shape proof.
pub(crate) const VOCABULARY_ARRAY_COUNT: usize = 3;

/// Conservative non-version JSON serialization budget for agent context.
pub(crate) const STATIC_SERIALIZED_UPPER_BOUND: usize = 8064;
