//! Module: action

use crate::action_specs::{write_action_table_rows, write_no_registered_actions};
use crate::app_impl::prelude::*;

pub(crate) fn write_action_registry_uninitialized(output: OutputFormat) {
    let message = "action registry is not initialized";
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            output,
        );
    }
}

pub(crate) fn write_action_registry(registry: &ActionRegistry, output: OutputFormat) -> ExitCode {
    let rows = action_table_rows(registry);
    if rows.is_empty() {
        return write_no_registered_actions(output);
    }

    if output == OutputFormat::Text {
        write_action_table_rows(&rows);
        return ExitCode::SUCCESS;
    }

    let actions: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "idempotency": row.idempotency,
                "retry_safety": row.retry_safety,
                "side_effect": row.side_effect,
                "input_slot_count": row.input_slot_count,
                "output_slot_count": row.output_slot_count,
                "timeout_ms": row.timeout_ms,
            })
        })
        .collect();
    emit_json_or_return!(
        &serde_json::json!({
            "success": true,
            "actions": actions,
        }),
        output,
    );
    ExitCode::SUCCESS
}

pub(crate) fn write_action_inspect(
    registry: &ActionRegistry,
    action_name: String,
    output: OutputFormat,
) -> ExitCode {
    match vb_core::action::ActionName::new(&action_name) {
        Ok(name) => match registry.resolve_by_name(&name) {
            Ok(contract) => write_action_contract(contract, output),
            Err(error) => write_action_inspect_error(&action_name, &error, output),
        },
        Err(e) => {
            let message = format!("invalid action name: {}", e);
            if output == OutputFormat::Text {
                errln!("{message}");
            } else {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "action_name": action_name,
                        "error": message,
                    }),
                    output,
                );
            }
            CliExitCode::ValidationFailed.into()
        }
    }
}

pub(crate) fn write_action_inspect_error(
    action_name: &str,
    error: &vb_core::action::ActionError,
    output: OutputFormat,
) -> ExitCode {
    let message = format!("action '{action_name}' is not registered: {error}");
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "action_name": action_name,
                "error": message,
            }),
            output,
        );
    }
    CliExitCode::ValidationFailed.into()
}

pub(crate) fn write_action_contract(
    contract: &vb_core::action::ActionContract,
    output: OutputFormat,
) -> ExitCode {
    let detail = action_contract_detail(contract);
    if output == OutputFormat::Text {
        write_action_contract_text(&detail);
    } else {
        emit_json_or_return!(&detail.to_json(), output);
    }
    ExitCode::SUCCESS
}

pub(crate) fn write_action_contract_text(detail: &ActionContractDetail) {
    outln!("action {}", detail.id);
    outln!("  input_slot_count: {}", detail.input_slot_count);
    outln!("  output_slot_count: {}", detail.output_slot_count);
    outln!("  max_input_bytes: {}", detail.max_input_bytes);
    outln!("  max_output_bytes: {}", detail.max_output_bytes);
    outln!("  timeout_ms: {}", detail.timeout_ms);
    outln!("  idempotency: {}", detail.idempotency);
    outln!("  retry_safety: {}", detail.retry_safety);
    outln!("  side_effect: {}", detail.side_effect);
    outln!("  idempotency_rule: {}", detail.idempotency_rule);
    outln!(
        "  required_capabilities: {}",
        detail.required_capabilities.join(",")
    );
    outln!("  failure_codes: {}", detail.failure_codes.join(","));
    outln!("  example_input_schema: {}", detail.example_input_schema);
    outln!("  example_output_schema: {}", detail.example_output_schema);
}

#[derive(Debug, Clone)]
pub(crate) struct ActionContractDetail {
    pub(crate) id: u16,
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
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": true,
            "action": {
                "id": self.id,
                "input_slot_count": self.input_slot_count,
                "output_slot_count": self.output_slot_count,
                "max_input_bytes": self.max_input_bytes,
                "max_output_bytes": self.max_output_bytes,
                "timeout_ms": self.timeout_ms,
                "idempotency": self.idempotency,
                "retry_safety": self.retry_safety,
                "side_effect": self.side_effect,
                "required_capabilities": self.required_capabilities,
                "failure_codes": self.failure_codes,
                "idempotency_rule": self.idempotency_rule,
                "example_input_schema": self.example_input_schema,
                "example_output_schema": self.example_output_schema,
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActionTableRow {
    pub(crate) id: u16,
    pub(crate) idempotency: &'static str,
    pub(crate) retry_safety: &'static str,
    pub(crate) side_effect: &'static str,
    pub(crate) input_slot_count: u16,
    pub(crate) output_slot_count: u16,
    pub(crate) timeout_ms: u64,
}
