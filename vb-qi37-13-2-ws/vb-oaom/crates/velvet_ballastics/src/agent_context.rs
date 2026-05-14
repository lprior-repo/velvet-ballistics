//! Agent context builder.
#![forbid(unsafe_code)]

use vb_runtime::action::ActionRegistry;

/// Build the agent context structure.
pub fn build_agent_context(registry: ActionRegistry) -> serde_json::Value {
    let _ = registry;
    serde_json::json!({
        "version": "1",
        "capabilities": [],
    })
}
