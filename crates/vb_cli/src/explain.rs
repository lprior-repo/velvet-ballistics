#![forbid(unsafe_code)]
//! Explain command main dispatch.

use crate::args::{
    ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget, VerifyProfile,
};
use crate::cli_envelope;
use crate::exit_code::CliExitCode;
use crate::explain_repair::explain_repair_hint;
use crate::explain_reports::{
    explain_compile_repair_hint, explain_gate_pass, explain_verification_failure,
    verify_error_message,
};
use crate::explain_validation::explain_validation_error;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_failure_message, write_stderr_line,
    write_stdout_line,
};
use crate::output_utils::*;
use vb_core::{CompiledNodeKind, StepIdx};
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn cmd_explain(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            write_failure_message(
                &format!("file is not valid UTF-8: {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Phase 1: YAML parse
    if let Err(e) = vb_yaml::parse_workflow_source(text) {
        if output == OutputFormat::Text {
            crate::outln!("YAML Parse Error:");
            crate::outln!("  {e}");
            crate::outln!("");
            explain_repair_hint(
                "yaml_parse",
                &[
                    "Check YAML syntax: use spaces for indentation, not tabs",
                    "Ensure all quotes are matched",
                    "Verify the file uses valid UTF-8 encoding",
                ],
            );
        } else {
            crate::emit_json_or_return!(
                &explain_failure_report(
                    "yaml_parse",
                    &format!("YAML parse error: {e}"),
                    &["Check YAML syntax: use spaces for indentation, not tabs"],
                    CliExitCode::ValidationFailed,
                ),
                output,
            );
        }
        return CliExitCode::ValidationFailed.into();
    }

    // Phase 2: Compilation
    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            if output == OutputFormat::Text {
                crate::outln!("Workflow has {} validation error(s):", errors.0.len());
                crate::outln!("");
                for (i, err) in errors.0.iter().enumerate() {
                    if i > 0 {
                        crate::outln!("---");
                    }
                    explain_error(err);
                }
            } else {
                let error_messages: Vec<String> = errors
                    .0
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();
                crate::emit_json_or_return!(
                    &explain_compile_failure_report(&error_messages),
                    output
                );
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Phase 3: Verification (runs all gates)
    match crate::commands_verify::run_verification(text, &bytes, VerifyProfile::Standard) {
        Ok(result) => {
            if output == OutputFormat::Text {
                crate::outln!("Workflow verification certificate:");
                crate::outln!("  status:  valid");
                crate::outln!("  digest:  {}", result.digest_hex);
                crate::outln!("  nodes:   {}", result.node_count);
                crate::outln!("");

                // Execution plan section
                explain_execution_plan(&compiled);

                crate::outln!("Passed gates ({}):", result.checks.len());
                for check in &result.checks {
                    explain_gate_pass(check);
                }
                if !result.warnings.is_empty() {
                    crate::outln!("");
                    crate::outln!("Warnings ({}):", result.warnings.len());
                    for warning in &result.warnings {
                        crate::outln!("  - {warning}");
                    }
                    crate::outln!("");
                    explain_repair_hint(
                        "verification_warnings",
                        &[
                            "Review warnings and address them before production use",
                            "Use 'vb verify --profile full' for exhaustive validation",
                        ],
                    );
                }
                crate::outln!("All gates passed. Workflow is correct and verifiable.");
            } else {
                crate::emit_json_or_return!(&explain_success_report(&result, &compiled), output);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = crate::commands_verify::exit_code_for_error(&err);
            if output == OutputFormat::Text {
                explain_verification_failure(&err);
            } else {
                crate::emit_json_or_return!(
                    &explain_verification_failure_report(&err, code),
                    output
                );
            }
            code.into()
        }
    }
}

pub(crate) fn explain_failure_report(
    phase: &'static str,
    message: &str,
    repair_hints: &[&'static str],
    code: CliExitCode,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": false,
        "status": "invalid",
        "phase": phase,
        "errors": [{ "phase": phase, "message": message }],
        "repair_hints": repair_hints,
        "exit_code": cli_exit_code_number(code)
    })
}

pub(crate) fn explain_compile_failure_report(errors: &[String]) -> serde_json::Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": false,
        "status": "invalid",
        "phase": "compile",
        "errors": errors,
        "repair_hints": ["Run validate to isolate syntax and schema errors"],
        "exit_code": cli_exit_code_number(CliExitCode::ValidationFailed)
    })
}

/// Emit execution plan with graph, resources, actions, suspension points, slots, and budget.
fn explain_execution_plan(compiled: &vb_core::CompiledWorkflow) {
    crate::outln!("Execution Plan:");
    crate::outln!("  entry:     step {}", compiled.entry().get());

    // Resources / budget
    let contract = compiled.resource_contract();
    crate::outln!("  budget:");
    crate::outln!("    max_steps:     {}", contract.max_steps);
    crate::outln!("    max_slots:     {}", contract.max_slots);
    crate::outln!("    max_constants: {}", contract.max_constants);
    crate::outln!("    max_accessors: {}", contract.max_accessors);
    crate::outln!("    max_expressions: {}", contract.max_expressions);
    crate::outln!("  slots:     {} slots", compiled.slot_count());
    crate::outln!("  steps:     {} nodes", compiled.node_count());

    // Collect actions and suspension points
    let mut actions = Vec::new();
    let mut suspension_points = Vec::new();

    for step in 0..compiled.node_count() {
        let step_idx = StepIdx::new(step);
        if let Some(node) = compiled.node(step_idx) {
            let name = compiled.step_name(step_idx).unwrap_or("<unnamed>");
            match node.kind {
                CompiledNodeKind::Do { .. } => {
                    actions.push(format!("step {} ({})", step, name));
                    suspension_points.push(format!("step {} ({}) - Do action", step, name));
                }
                CompiledNodeKind::Ask { .. } => {
                    suspension_points.push(format!("step {} ({}) - Ask for input", step, name));
                }
                CompiledNodeKind::WaitUntil { .. } => {
                    suspension_points.push(format!("step {} ({}) - WaitUntil deadline", step, name));
                }
                CompiledNodeKind::WaitEvent { .. } => {
                    suspension_points.push(format!("step {} ({}) - WaitEvent", step, name));
                }
                CompiledNodeKind::TogetherStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Parallel branches", step, name));
                }
                CompiledNodeKind::ForEachStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - ForEach loop", step, name));
                }
                CompiledNodeKind::CollectStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Collect", step, name));
                }
                CompiledNodeKind::ReduceStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Reduce", step, name));
                }
                CompiledNodeKind::RepeatStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Repeat loop", step, name));
                }
                _ => {}
            }
        }
    }

    if !actions.is_empty() {
        crate::outln!("  actions:   {} action(s)", actions.len());
        for action in &actions {
            crate::outln!("    - {}", action);
        }
    } else {
        crate::outln!("  actions:   none");
    }

    if !suspension_points.is_empty() {
        crate::outln!("  suspension_points: {} point(s)", suspension_points.len());
        for sp in &suspension_points {
            crate::outln!("    - {}", sp);
        }
    } else {
        crate::outln!("  suspension_points: none");
    }
}

pub(crate) fn explain_success_report(
    result: &crate::commands_verify::VerifyOk,
    compiled: &vb_core::CompiledWorkflow,
) -> serde_json::Value {
    let contract = compiled.resource_contract();

    // Collect actions and suspension points for JSON
    let mut actions = Vec::new();
    let mut suspension_points = Vec::new();

    for step in 0..compiled.node_count() {
        let step_idx = StepIdx::new(step);
        if let Some(node) = compiled.node(step_idx) {
            let name = compiled.step_name(step_idx).unwrap_or("<unnamed>").to_string();
            match node.kind {
                CompiledNodeKind::Do { .. } => {
                    actions.push(format!("step {} ({})", step, name));
                    suspension_points.push(format!("step {} ({}) - Do action", step, name));
                }
                CompiledNodeKind::Ask { .. } => {
                    suspension_points.push(format!("step {} ({}) - Ask for input", step, name));
                }
                CompiledNodeKind::WaitUntil { .. } => {
                    suspension_points.push(format!("step {} ({}) - WaitUntil deadline", step, name));
                }
                CompiledNodeKind::WaitEvent { .. } => {
                    suspension_points.push(format!("step {} ({}) - WaitEvent", step, name));
                }
                CompiledNodeKind::TogetherStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Parallel branches", step, name));
                }
                CompiledNodeKind::ForEachStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - ForEach loop", step, name));
                }
                CompiledNodeKind::CollectStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Collect", step, name));
                }
                CompiledNodeKind::ReduceStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Reduce", step, name));
                }
                CompiledNodeKind::RepeatStart { .. } => {
                    suspension_points.push(format!("step {} ({}) - Repeat loop", step, name));
                }
                _ => {}
            }
        }
    }

    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": true,
        "status": "valid",
        "artifact": {
            "ir_digest_hex": result.digest_hex.as_str(),
            "node_count": result.node_count
        },
        "execution_plan": {
            "entry_step": compiled.entry().get(),
            "slots": compiled.slot_count(),
            "budget": {
                "max_steps": contract.max_steps,
                "max_slots": contract.max_slots,
                "max_constants": contract.max_constants,
                "max_accessors": contract.max_accessors,
                "max_expressions": contract.max_expressions
            },
            "actions": actions,
            "suspension_points": suspension_points
        },
        "passed_gates": &result.checks,
        "warnings": &result.warnings,
        "repair_hints": [],
        "exit_code": cli_exit_code_number(CliExitCode::Success)
    })
}

pub(crate) fn explain_verification_failure_report(
    err: &crate::commands_verify::VerifyError,
    code: CliExitCode,
) -> serde_json::Value {
    let message = verify_error_message(err);
    explain_failure_report(
        "verification",
        &message,
        &["Run verify --profile full for details"],
        code,
    )
}

pub(crate) fn explain_error(err: &vb_compile::CompileError) {
    use vb_compile::CompileError;
    match err {
        CompileError::SourceTooLarge { actual, limit } => {
            crate::outln!("Source Too Large");
            crate::outln!(
                "  The workflow YAML source is {actual} bytes, exceeds limit of {limit}."
            );
        }
        CompileError::EmptySource => {
            crate::outln!("Empty Source");
            crate::outln!("  The workflow file contains no YAML document.");
        }
        CompileError::Parse(e) => {
            crate::outln!("YAML Parse Error");
            crate::outln!("  The YAML parser rejected the document: {e}");
        }
        CompileError::DocumentCount { count } => {
            crate::outln!("Multiple YAML Documents");
            crate::outln!("  Expected exactly one YAML document, but found {count}.");
        }
        CompileError::TopLevelNotMapping => {
            crate::outln!("Invalid Top-Level Structure");
            crate::outln!("  The top-level YAML document must be a mapping.");
        }
        CompileError::NonStringKey { mark } => {
            crate::outln!("Non-String Key");
            crate::outln!("  A mapping key at position {mark:?} is not a string.");
        }
        CompileError::DuplicateKey { key, mark } => {
            crate::outln!("Duplicate Key");
            crate::outln!("  The YAML mapping contains duplicate key '{key}' at {mark:?}.");
        }
        CompileError::AliasForbidden { mark } => {
            crate::outln!("YAML Alias Forbidden");
            crate::outln!("  YAML aliases are not allowed at {mark:?}.");
        }
        CompileError::AnchorForbidden { mark } => {
            crate::outln!("YAML Anchor Forbidden");
            crate::outln!("  YAML anchors are not allowed at {mark:?}.");
        }
        CompileError::MergeKeyForbidden { mark } => {
            crate::outln!("YAML Merge Key Forbidden");
            crate::outln!("  YAML merge keys are not allowed at {mark:?}.");
        }
        CompileError::TagForbidden { mark } => {
            crate::outln!("YAML Tag Forbidden");
            crate::outln!("  YAML tags are not allowed at {mark:?}.");
        }
        CompileError::BadValue => {
            crate::outln!("Invalid YAML Scalar");
            crate::outln!("  A YAML scalar value is malformed.");
        }
        CompileError::FloatForbidden => {
            crate::outln!("Floating-Point Numbers Forbidden");
            crate::outln!("  Floating-point YAML scalars are not allowed.");
        }
        CompileError::DepthLimit { depth, limit } => {
            crate::outln!("Nesting Depth Exceeded");
            crate::outln!("  YAML nesting depth of {depth} exceeds limit of {limit}.");
        }
        CompileError::NodeLimit { limit } => {
            crate::outln!("YAML Node Limit Exceeded");
            crate::outln!("  The workflow exceeds node limit of {limit}.");
        }
        CompileError::SequenceLimit { actual, limit } => {
            crate::outln!("Sequence Too Long");
            crate::outln!("  A sequence has {actual} items, exceeding limit of {limit}.");
        }
        CompileError::MappingLimit { actual, limit } => {
            crate::outln!("Mapping Too Large");
            crate::outln!("  A mapping has {actual} entries, exceeding limit of {limit}.");
        }
        CompileError::ScalarLimit { actual, limit } => {
            crate::outln!("Scalar Too Long");
            crate::outln!("  A scalar is {actual} chars, exceeding limit of {limit}.");
        }
        CompileError::MissingField { field } => {
            crate::outln!("Missing Required Field");
            crate::outln!("  Required workflow field '{field}' is missing.");
        }
        CompileError::UnknownTopLevelField { field } => {
            crate::outln!("Unknown Workflow Field");
            crate::outln!("  '{field}' is not a recognized Velvet workflow field.");
        }
        CompileError::InvalidVersion { actual } => {
            crate::outln!("Invalid Workflow Version");
            crate::outln!(
                "  Found version '{actual}', but Velvet v1 requires 'velvet-ballistics/v1'."
            );
        }
        CompileError::InvalidTriggerCount { count } => {
            crate::outln!("Invalid Trigger Count");
            crate::outln!("  Workflow must declare exactly one trigger, but found {count}.");
        }
        CompileError::UnknownTriggerKind { trigger } => {
            crate::outln!("Unknown Trigger Kind");
            crate::outln!("  Trigger kind '{trigger}' is not recognized.");
        }
        CompileError::TriggerShape {
            trigger,
            expected: _,
        } => {
            crate::outln!("Invalid Trigger Shape");
            crate::outln!("  Trigger '{trigger}' has the wrong structure.");
        }
        CompileError::UnknownTriggerField { trigger, field } => {
            crate::outln!("Unknown Trigger Field");
            crate::outln!("  Trigger '{trigger}' has unknown field '{field}'.");
        }
        CompileError::MissingTriggerField { trigger, field } => {
            crate::outln!("Missing Trigger Field");
            crate::outln!("  Trigger '{trigger}' is missing required field '{field}'.");
        }
        CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: _,
        } => {
            crate::outln!("Invalid Trigger Field");
            crate::outln!("  Trigger '{trigger}' field '{field}' is invalid.");
        }
        CompileError::FieldShape { field, expected: _ } => {
            crate::outln!("Invalid Field Shape");
            crate::outln!("  Field '{field}' has the wrong structure.");
        }
        CompileError::UnknownInputSchemaField { field } => {
            crate::outln!("Unknown Input Schema Field");
            crate::outln!("  '{field}' is not a recognized input schema field.");
        }
        CompileError::InvalidInputSchema { field, expected: _ } => {
            crate::outln!("Invalid Input Schema");
            crate::outln!("  Input schema field '{field}' is invalid.");
        }
        CompileError::UnsupportedTopLevelResult => {
            crate::outln!("Unsupported Top-Level Result");
            crate::outln!("  Non-empty top-level result mapping is not supported.");
        }
        CompileError::EmptySteps => {
            crate::outln!("Empty Steps");
            crate::outln!("  Workflow must contain at least one executable step.");
        }
        CompileError::InvalidName { field, value } => {
            crate::outln!("Invalid Name");
            crate::outln!("  '{value}' is not a valid Velvet v1 name for {field}.");
        }
        CompileError::MissingStepId { step } => {
            crate::outln!("Missing Step ID");
            crate::outln!("  Step at index {step} is missing its required 'id' field.");
        }
        CompileError::DuplicateStepId { id } => {
            crate::outln!("Duplicate Step ID");
            crate::outln!("  Step ID '{id}' appears more than once in the workflow.");
        }
        CompileError::StepShape { step } => {
            crate::outln!("Invalid Step Shape");
            crate::outln!("  Step at index {step} must be a YAML mapping.");
        }
        CompileError::UnknownStepField { step, field } => {
            crate::outln!("Unknown Step Field");
            crate::outln!("  Step {step} has unknown field '{field}'.");
        }
        CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field,
        } => {
            crate::outln!("Unknown Primitive Field");
            crate::outln!("  Step {step} primitive '{primitive}' has unknown field '{field}'.");
        }
        CompileError::MissingStepPrimitive { step } => {
            crate::outln!("Missing Step Primitive");
            crate::outln!("  Step {step} is missing a primitive action.");
        }
        CompileError::MultipleStepPrimitives { step } => {
            crate::outln!("Multiple Step Primitives");
            crate::outln!("  Step {step} contains multiple primitive fields.");
        }
        CompileError::UnsupportedStepPrimitive { step, primitive } => {
            crate::outln!("Unsupported Step Primitive");
            crate::outln!("  Step {step} primitive '{primitive}' is not supported.");
        }
        CompileError::UnsupportedStepControlField { step, field } => {
            crate::outln!("Unsupported Step Control Field");
            crate::outln!("  Step {step} control field '{field}' is not supported.");
        }
        CompileError::MissingStepField { step, field } => {
            crate::outln!("Missing Step Field");
            crate::outln!("  Step {step} is missing required field '{field}'.");
        }
        CompileError::StepFieldShape {
            step,
            field,
            expected: _,
        } => {
            crate::outln!("Invalid Step Field Shape");
            crate::outln!("  Step {step} field '{field}' has wrong structure.");
        }
        CompileError::StepIndexOutOfRange { value } => {
            crate::outln!("Step Index Out of Range");
            crate::outln!("  Step index {value} exceeds the u16 representation limit.");
        }
        CompileError::SlotIndexOutOfRange { value } => {
            crate::outln!("Slot Index Out of Range");
            crate::outln!("  Slot index {value} is outside the valid u16 range.");
        }
        CompileError::BranchTargetOutOfRange { value } => {
            crate::outln!("Branch Target Out of Range");
            crate::outln!("  Branch target {value} is outside the valid u16 range.");
        }
        CompileError::BackwardBranchTarget { step, target } => {
            crate::outln!("Backward Branch Target");
            crate::outln!("  Step {step} branches to {target}, but forward branches are required.");
        }
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        } => {
            crate::outln!("Primitive Limit Exceeded");
            crate::outln!(
                "  Primitive '{primitive}' field '{field}' value {value} exceeds limit {limit}."
            );
        }
        CompileError::LastStepMustFinish => {
            crate::outln!("Last Step Must Finish");
            crate::outln!("  The final step in a linear workflow must be a 'finish' step.");
        }
        CompileError::UnsupportedConstantValue { step } => {
            crate::outln!("Unsupported Constant Value");
            crate::outln!("  Step {step} constant value must be a scalar YAML value.");
        }
        CompileError::UnknownReferenceRoot { reference, root } => {
            crate::outln!("Unknown Reference Root");
            crate::outln!("  Reference '{reference}' uses unknown root '{root}'.");
        }
        CompileError::IllegalReference { reference } => {
            crate::outln!("Illegal Reference");
            crate::outln!("  Reference '{reference}' is not allowed in deterministic workflows.");
        }
        CompileError::UnknownReferenceName {
            kind,
            reference,
            name,
        } => {
            crate::outln!("Unknown Reference");
            crate::outln!("  Reference '{reference}' refers to unknown {kind} '{name}'.");
        }
        CompileError::UnsupportedAccessorReference {
            reference,
            root,
            path,
        } => {
            crate::outln!("Unsupported Accessor Reference");
            crate::outln!(
                "  Accessor reference '{reference}' (root: {root}, path: {path}) is not supported."
            );
        }
        CompileError::UnknownStepTarget { step, target } => {
            crate::outln!("Unknown Step Target");
            crate::outln!("  Step {step} branches to undeclared step index {target}.");
        }
        CompileError::UnreachableStep { step } => {
            crate::outln!("Unreachable Step");
            crate::outln!("  Step {step} cannot be reached from the workflow entry point.");
        }
        CompileError::TypeMismatch {
            field,
            expected,
            found,
        } => {
            crate::outln!("Type Mismatch");
            crate::outln!("  Field '{field}': expected {expected}, but found {found}.");
        }
        CompileError::Workflow(e) => {
            crate::outln!("Workflow IR Validation Error");
            crate::outln!("  {e}");
        }
        CompileError::Validation(e) => {
            explain_validation_error(e);
        }
        _ => {
            crate::outln!("Compilation Error");
            crate::outln!("  {err}");
        }
    }
    explain_compile_repair_hint(err);
}
