//! Action table rows, contract specs, and CLI action registration.

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use std::process::ExitCode;

use crate::output::json_error;

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

pub(crate) struct ActionTableRow {
    pub(crate) id: u16,
    pub(crate) idempotency: &'static str,
    pub(crate) retry_safety: &'static str,
    pub(crate) side_effect: &'static str,
    pub(crate) input_slot_count: u16,
    pub(crate) output_slot_count: u16,
    pub(crate) timeout_ms: u64,
}

pub(crate) fn action_table_rows(
    contracts: &[vb_core::action::ActionContract],
) -> Vec<ActionTableRow> {
    contracts
        .iter()
        .map(|contract| ActionTableRow {
            id: contract.id.get(),
            idempotency: action_idempotency_name(contract.idempotency),
            retry_safety: action_retry_safety_name(contract.retry_safety),
            side_effect: action_side_effect_name(contract.side_effect),
            input_slot_count: contract.input_slot_count,
            output_slot_count: contract.output_slot_count,
            timeout_ms: contract.timeout_ms,
        })
        .collect()
}

pub(crate) fn action_contract_detail(
    contract: &vb_core::action::ActionContract,
) -> ActionContractDetail {
    ActionContractDetail {
        id: contract.id.get(),
        name: contract.name.to_string(),
        input_slot_count: contract.input_slot_count,
        output_slot_count: contract.output_slot_count,
        max_input_bytes: contract.max_input_bytes,
        max_output_bytes: contract.max_output_bytes,
        timeout_ms: contract.timeout_ms,
        idempotency: action_idempotency_name(contract.idempotency),
        retry_safety: action_retry_safety_name(contract.retry_safety),
        side_effect: action_side_effect_name(contract.side_effect),
        required_capabilities: contract
            .required_capabilities
            .iter()
            .map(|capability| format!("{}:{}", capability.name(), capability.action_id().get()))
            .collect(),
        failure_codes: action_failure_code_names().to_vec(),
        idempotency_rule: action_idempotency_rule(contract.idempotency, contract.retry_safety),
        example_input_schema: "postcard(ActionInput { run, step, action, input, ticket })",
        example_output_schema: "postcard(ActionOutcome::Ready|Suspended|Failed)",
    }
}

pub(crate) fn write_action_table_rows(rows: &[ActionTableRow]) {
    crate::outln!(
        "id\tidempotency\tretry_safety\tside_effect\tinput_slots\toutput_slots\ttimeout_ms"
    );
    rows.iter().for_each(|row| {
        crate::outln!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.id,
            row.idempotency,
            row.retry_safety,
            row.side_effect,
            row.input_slot_count,
            row.output_slot_count,
            row.timeout_ms
        );
    });
}

pub(crate) fn write_no_registered_actions(output: OutputFormat) -> ExitCode {
    let message = "no registered actions";
    if output == OutputFormat::Text {
        crate::outln!("{message}");
        ExitCode::SUCCESS
    } else {
        crate::emit_json_or_return!(
            &serde_json::json!({
                "success": true,
                "actions": [],
                "message": message,
            }),
            output,
        );
        ExitCode::SUCCESS
    }
}

pub(crate) fn registered_cli_actions()
-> vb_core::action::ActionResult<Vec<vb_core::action::ActionContract>> {
    cli_action_specs()
        .iter()
        .map(|spec| action_contract(*spec))
        .collect()
}

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

pub(crate) fn cli_action_specs() -> &'static [CliActionSpec] {
    &[
        CliActionSpec {
            id: 1,
            idempotency: vb_core::action::Idempotency::DeterministicPure,
            retry_safety: vb_core::action::RetrySafety::Idempotent,
            side_effect: vb_core::action::SideEffect::Pure,
            input_slot_count: 1,
            output_slot_count: 1,
            timeout_ms: 1_000,
        },
        CliActionSpec {
            id: 2,
            idempotency: vb_core::action::Idempotency::IdempotentExternal,
            retry_safety: vb_core::action::RetrySafety::RequiresIdempotencyKey,
            side_effect: vb_core::action::SideEffect::LocalWrite,
            input_slot_count: 2,
            output_slot_count: 1,
            timeout_ms: 5_000,
        },
        CliActionSpec {
            id: 3,
            idempotency: vb_core::action::Idempotency::AtLeastOnceExternal,
            retry_safety: vb_core::action::RetrySafety::NotRetrySafe,
            side_effect: vb_core::action::SideEffect::ExternalWrite,
            input_slot_count: 1,
            output_slot_count: 0,
            timeout_ms: 10_000,
        },
    ]
}

pub(crate) fn action_contract(
    spec: CliActionSpec,
) -> vb_core::action::ActionResult<vb_core::action::ActionContract> {
    let name_str: &str = match spec.id {
        1 => "validate",
        2 => "write",
        3 => "run",
        _ => "unknown",
    };
    let name = vb_core::action::ActionName::new(name_str)
        .map_err(|_error| vb_core::action::ActionError::DispatchFailed)?;
    Ok(vb_core::action::ActionContract {
        id: vb_core::ActionId::new(spec.id),
        name,
        input_slot_count: spec.input_slot_count,
        output_slot_count: spec.output_slot_count,
        max_input_bytes: 65_536,
        max_output_bytes: 65_536,
        timeout_ms: spec.timeout_ms,
        idempotency: spec.idempotency,
        side_effect: spec.side_effect,
        retry_safety: spec.retry_safety,
        required_capabilities: Box::new([]),
    })
}

pub(crate) fn action_idempotency_name(value: vb_core::action::Idempotency) -> &'static str {
    match value {
        vb_core::action::Idempotency::DeterministicPure => "deterministic_pure",
        vb_core::action::Idempotency::IdempotentExternal => "idempotent_external",
        vb_core::action::Idempotency::AtLeastOnceExternal => "at_least_once_external",
        _ => "unknown",
    }
}

pub(crate) fn action_retry_safety_name(value: vb_core::action::RetrySafety) -> &'static str {
    match value {
        vb_core::action::RetrySafety::Idempotent => "idempotent",
        vb_core::action::RetrySafety::RequiresIdempotencyKey => "requires_idempotency_key",
        vb_core::action::RetrySafety::NotRetrySafe => "not_retry_safe",
        vb_core::action::RetrySafety::Unknown => "unknown",
        // RetrySafety is `#[non_exhaustive]`; future variants default to "unknown".
        _ => "unknown",
    }
}

pub(crate) fn action_side_effect_name(value: vb_core::action::SideEffect) -> &'static str {
    match value {
        vb_core::action::SideEffect::Pure => "pure",
        vb_core::action::SideEffect::LocalRead => "local_read",
        vb_core::action::SideEffect::LocalWrite => "local_write",
        vb_core::action::SideEffect::ExternalRead => "external_read",
        vb_core::action::SideEffect::ExternalWrite => "external_write",
        vb_core::action::SideEffect::Process => "process",
        vb_core::action::SideEffect::UnsafeShell => "unsafe_shell",
        _ => "unknown",
    }
}

pub(crate) fn action_failure_code_names() -> &'static [&'static str] {
    &[
        "rejected",
        "timeout",
        "rate_limited",
        "resource_exhausted",
        "external_unavailable",
        "invalid_input",
        "permission_denied",
        "conflict",
        "unknown",
    ]
}

pub(crate) fn action_idempotency_rule(
    idempotency: vb_core::action::Idempotency,
    retry_safety: vb_core::action::RetrySafety,
) -> &'static str {
    match (idempotency, retry_safety) {
        (vb_core::action::Idempotency::DeterministicPure, _) => {
            "pure deterministic actions may replay without an external key"
        }
        (_, vb_core::action::RetrySafety::RequiresIdempotencyKey) => {
            "external retries require a stable idempotency key"
        }
        (_, vb_core::action::RetrySafety::NotRetrySafe) => {
            "unsafe actions must not be retried automatically"
        }
        _ => "retry behavior follows the action contract",
    }
}

pub(crate) fn write_action_registry_error(
    error: &vb_core::action::ActionError,
    output: OutputFormat,
) -> std::process::ExitCode {
    let message = format!("failed to register CLI action contracts: {error}");
    if output == OutputFormat::Text {
        crate::errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            CliExitCode::ValidationFailed,
            output,
        );
    }
    crate::exit_code::CliExitCode::ValidationFailed.into()
}

pub(crate) fn write_action_registry_uninitialized(output: OutputFormat) {
    let message = "action registry is not initialized";
    if output == OutputFormat::Text {
        crate::errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            CliExitCode::ValidationFailed,
            output,
        );
    }
}

pub(crate) fn write_action_registry(
    contracts: &[vb_core::action::ActionContract],
    output: OutputFormat,
) -> std::process::ExitCode {
    let rows = action_table_rows(contracts);
    if rows.is_empty() {
        return write_no_registered_actions(output);
    }

    if output == OutputFormat::Text {
        write_action_table_rows(&rows);
        return std::process::ExitCode::SUCCESS;
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
    crate::emit_json_or_return!(
        &serde_json::json!({
            "success": true,
            "actions": actions,
        }),
        output,
    );
    std::process::ExitCode::SUCCESS
}

pub(crate) fn write_action_inspect(
    contracts: &[vb_core::action::ActionContract],
    action_name: &vb_core::action::ActionName,
    output: OutputFormat,
) -> std::process::ExitCode {
    match contracts.iter().find(|c| c.name == *action_name) {
        Some(contract) => write_action_contract_json(contract, output),
        None => write_action_inspect_error(
            action_name.as_str(),
            &vb_core::action::ActionError::UnknownAction {
                action: vb_core::ids::ActionId::new(0),
            },
            output,
        ),
    }
}

fn write_action_contract_json(
    contract: &vb_core::action::ActionContract,
    output: OutputFormat,
) -> std::process::ExitCode {
    let detail = action_contract_detail(contract);
    if output == OutputFormat::Text {
        write_action_contract_text(&detail);
    } else {
        crate::emit_json_or_return!(&detail.to_json(), output);
    }
    std::process::ExitCode::SUCCESS
}

fn write_action_contract_text(detail: &ActionContractDetail) {
    crate::outln!("action {} ({})", detail.id, detail.name);
    crate::outln!("  input_slot_count: {}", detail.input_slot_count);
    crate::outln!("  output_slot_count: {}", detail.output_slot_count);
    crate::outln!("  max_input_bytes: {}", detail.max_input_bytes);
    crate::outln!("  max_output_bytes: {}", detail.max_output_bytes);
    crate::outln!("  timeout_ms: {}", detail.timeout_ms);
    crate::outln!("  idempotency: {}", detail.idempotency);
    crate::outln!("  retry_safety: {}", detail.retry_safety);
    crate::outln!("  side_effect: {}", detail.side_effect);
    crate::outln!("  idempotency_rule: {}", detail.idempotency_rule);
    crate::outln!(
        "  required_capabilities: {}",
        detail.required_capabilities.join(",")
    );
    crate::outln!("  failure_codes: {}", detail.failure_codes.join(","));
    crate::outln!("  example_input_schema: {}", detail.example_input_schema);
    crate::outln!("  example_output_schema: {}", detail.example_output_schema);
}

fn write_action_inspect_error(
    action_name: &str,
    error: &vb_core::action::ActionError,
    output: OutputFormat,
) -> std::process::ExitCode {
    let message = format!("action '{action_name}' is not registered: {error}");
    if output == OutputFormat::Text {
        crate::errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "action_name": action_name,
                "error": message,
            }),
            CliExitCode::ValidationFailed,
            output,
        );
    }
    crate::exit_code::CliExitCode::ValidationFailed.into()
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety CLI string mapping tests (Tier 2).
// These tests assert the canonical 4-string mapping:
//   Idempotent                 → "idempotent"
//   RequiresIdempotencyKey    → "requires_idempotency_key"
//   NotRetrySafe               → "not_retry_safe"
//   Unknown                    → "unknown"
//
// On 3-variant code (Safe/KeyRequired/Unsafe), the strings are
// "safe"/"key_required"/"unsafe" — all 3 of the asserted literals
// differ. Tier 2 runtime fail (not compile fail) for the Idempotent,
// requires_idempotency_key, not_retry_safe assertions; Tier 1 compile
// fail for the Unknown assertion (the variant does not exist).
// =========================================================================

#[cfg(test)]
#[allow(clippy::doc_markdown)]
mod tests {
    use super::action_retry_safety_name;
    use vb_core::action::RetrySafety;

    /// Tier 2: `action_retry_safety_name(RetrySafety::Idempotent) == "idempotent"`.
    /// On 3-variant code: returns "safe"; on 4-variant code: returns "idempotent".
    #[test]
    fn action_retry_safety_name_idempotent_returns_idempotent_literal() {
        let s = action_retry_safety_name(RetrySafety::Idempotent);
        assert_eq!(s, "idempotent");
    }

    /// Tier 2: `action_retry_safety_name(RetrySafety::RequiresIdempotencyKey) == "requires_idempotency_key"`.
    /// On 3-variant code: returns "key_required"; on 4-variant code: returns "requires_idempotency_key".
    #[test]
    fn action_retry_safety_name_requires_key_returns_requires_idempotency_key_literal() {
        let s = action_retry_safety_name(RetrySafety::RequiresIdempotencyKey);
        assert_eq!(s, "requires_idempotency_key");
    }

    /// Tier 2: `action_retry_safety_name(RetrySafety::NotRetrySafe) == "not_retry_safe"`.
    /// On 3-variant code: returns "unsafe"; on 4-variant code: returns "not_retry_safe".
    #[test]
    fn action_retry_safety_name_not_retry_safe_returns_not_retry_safe_literal() {
        let s = action_retry_safety_name(RetrySafety::NotRetrySafe);
        assert_eq!(s, "not_retry_safe");
    }

    /// Tier 1: `action_retry_safety_name(RetrySafety::Unknown) == "unknown"`.
    /// On 3-variant code: `RetrySafety::Unknown` does not exist; compile fail.
    #[test]
    fn action_retry_safety_name_unknown_returns_unknown_literal() {
        let s = action_retry_safety_name(RetrySafety::Unknown);
        assert_eq!(s, "unknown");
    }

    /// Tier 1: the 4 strings are pairwise distinct (set size = 4).
    #[test]
    fn action_retry_safety_name_4_strings_pairwise_distinct() {
        let s_idempotent = action_retry_safety_name(RetrySafety::Idempotent);
        let s_requires_key = action_retry_safety_name(RetrySafety::RequiresIdempotencyKey);
        let s_not_retry_safe = action_retry_safety_name(RetrySafety::NotRetrySafe);
        let s_unknown = action_retry_safety_name(RetrySafety::Unknown);
        let set: std::collections::BTreeSet<&str> =
            [s_idempotent, s_requires_key, s_not_retry_safe, s_unknown]
                .into_iter()
                .collect();
        assert_eq!(
            set.len(),
            4,
            "4 strings must be pairwise distinct; got set {set:?}"
        );
    }
}
