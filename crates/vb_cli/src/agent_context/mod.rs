#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code, unused_mut, unused_variables))]

mod constants;
mod flags;
mod policy;
mod primitives;

mod commands;

pub(crate) use constants::*;

use serde_json::Value;

/// Build the machine-readable CLI surface for AI agents.
pub(crate) fn build(version: &str) -> Value {
    serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": AGENT_CONTEXT_KIND,
        "cli": CLI_NAME,
        "binary_aliases": policy::binary_aliases(),
        "version": version,
        "language_version": LANGUAGE_VERSION,
        "agent_contract": policy::agent_contract(),
        "vocabulary_policy": policy::vocabulary_policy(),
        "active_gates": policy::active_gates(),
        "known_blockers": policy::known_blockers(),
        "exit_codes": policy::exit_codes(),
        "enums": policy::enums(),
        "commands": commands::commands(),
        "planned_agent_primitives": primitives::planned_agent_primitives()
    })
}

#[cfg(kani)]
pub(crate) mod kani_shape;

#[cfg(any(test, kani))]
mod tests;
