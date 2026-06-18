//! Action-spec type definitions.

/// A tabular row describing an action contract's operational boundaries.
pub(crate) struct ActionTableRow {
    pub(crate) id: u16,
    pub(crate) idempotency: &'static str,
    pub(crate) retry_safety: &'static str,
    pub(crate) side_effect: &'static str,
    pub(crate) input_slot_count: u16,
    pub(crate) output_slot_count: u16,
    pub(crate) timeout_ms: u64,
}

/// A full-detail record for a single action contract, including schemas and rules.
pub(crate) struct ActionContractDetail {
    pub(crate) id: u16,
    pub(crate) name: String,
    pub(crate) input_slot_count: u16,
    pub(crate) output_slot_count: u16,
    pub(crate) max_input_bytes: u32,
    pub(crate) max_output_bytes: u32,
    pub(crate) timeout_ms: u64,
    pub(crate) idempotency: &'static str,
    pub(crate) retry_safety: &'static str,
    pub(crate) side_effect: &'static str,
    pub(crate) required_capabilities: Vec<String>,
    pub(crate) failure_codes: Vec<&'static str>,
    pub(crate) idempotency_rule: &'static str,
    pub(crate) example_input_schema: &'static str,
    pub(crate) example_output_schema: &'static str,
}

impl ActionContractDetail {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": true,
            "action": {
                "id": self.id,
                "name": self.name.clone(),
                "input_slot_count": self.input_slot_count,
                "output_slot_count": self.output_slot_count,
                "max_input_bytes": self.max_input_bytes,
                "max_output_bytes": self.max_output_bytes,
                "timeout_ms": self.timeout_ms,
                "idempotency": self.idempotency,
                "retry_safety": self.retry_safety,
                "side_effect": self.side_effect,
                "required_capabilities": self.required_capabilities.clone(),
                "failure_codes": self.failure_codes.to_vec(),
                "idempotency_rule": self.idempotency_rule,
                "example_input_schema": self.example_input_schema,
                "example_output_schema": self.example_output_schema,
            }
        })
    }
}

/// A CLI-only action specification: the static blueprint from which contracts are built.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CliActionSpec {
    pub(crate) id: u16,
    pub(crate) idempotency: vb_core::action::Idempotency,
    pub(crate) retry_safety: vb_core::action::RetrySafety,
    pub(crate) side_effect: vb_core::action::SideEffect,
    pub(crate) input_slot_count: u16,
    pub(crate) output_slot_count: u16,
    pub(crate) timeout_ms: u64,
}
