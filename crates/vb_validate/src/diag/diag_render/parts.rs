#![forbid(unsafe_code)]
//! Stable diagnostic code and message mapping.

use crate::ValidationError;
use crate::diag::diag_codes::*;
use vb_core::diagnostic::DiagnosticCode;

mod contract;

use contract::contract_parts;

pub(super) fn error_diagnostic_parts(error: &ValidationError) -> (DiagnosticCode, String) {
    if let Some(parts) = schema_parts(error) {
        return parts;
    }
    if let Some(parts) = control_parts(error) {
        return parts;
    }
    if let Some(parts) = limit_parts(error) {
        return parts;
    }
    if let Some(parts) = gate_parts(error) {
        return parts;
    }
    contract_parts(error)
}

fn code_msg(code: u16, message: impl Into<String>) -> (DiagnosticCode, String) {
    (DiagnosticCode::new(code), message.into())
}

fn schema_parts(error: &ValidationError) -> Option<(DiagnosticCode, String)> {
    let parts = match error {
        ValidationError::DuplicateKey => code_msg(CODE_DUPLICATE_KEY, "duplicate key"),
        ValidationError::ForbiddenYamlFeature => {
            code_msg(CODE_FORBIDDEN_YAML_FEATURE, "forbidden YAML feature")
        }
        ValidationError::UnknownTopLevelField => {
            code_msg(CODE_UNKNOWN_TOP_LEVEL_FIELD, "unknown top-level field")
        }
        ValidationError::UnknownStepField => {
            code_msg(CODE_UNKNOWN_STEP_FIELD, "unknown step field")
        }
        ValidationError::MissingRequiredField { field } => code_msg(
            CODE_MISSING_REQUIRED_FIELD,
            format!("missing required field: {field}"),
        ),
        ValidationError::InvalidVersion { version } => {
            code_msg(CODE_INVALID_VERSION, format!("invalid version: {version}"))
        }
        ValidationError::InvalidId { id } => code_msg(CODE_INVALID_ID, format!("invalid ID: {id}")),
        ValidationError::ReservedId { id } => {
            code_msg(CODE_RESERVED_ID, format!("reserved ID: {id}"))
        }
        ValidationError::DuplicateId { id } => {
            code_msg(CODE_DUPLICATE_ID, format!("duplicate ID: {id}"))
        }
        ValidationError::MultipleStepPrimitives => {
            code_msg(CODE_MULTIPLE_STEP_PRIMITIVES, "multiple step primitives")
        }
        ValidationError::MissingStepPrimitive => {
            code_msg(CODE_MISSING_STEP_PRIMITIVE, "missing step primitive")
        }
        ValidationError::UnknownReference { reference } => code_msg(
            CODE_UNKNOWN_REFERENCE,
            format!("unknown reference: {reference}"),
        ),
        ValidationError::FutureReference { reference } => code_msg(
            CODE_FUTURE_REFERENCE,
            format!("future reference: {reference}"),
        ),
        ValidationError::SecretNotDeclared { secret } => code_msg(
            CODE_SECRET_NOT_DECLARED,
            format!("secret not declared: {secret}"),
        ),
        ValidationError::DirectRuntimeReference => {
            code_msg(CODE_DIRECT_RUNTIME_REFERENCE, "direct runtime reference")
        }
        _ => return None,
    };
    Some(parts)
}

fn control_parts(error: &ValidationError) -> Option<(DiagnosticCode, String)> {
    let parts = match error {
        ValidationError::InvalidThenTarget => {
            code_msg(CODE_INVALID_THEN_TARGET, "invalid then target")
        }
        ValidationError::ControlFlowCycle => {
            code_msg(CODE_CONTROL_FLOW_CYCLE, "control-flow cycle")
        }
        ValidationError::UnreachableStep { step } => {
            code_msg(CODE_UNREACHABLE_STEP, format!("unreachable step: {step}"))
        }
        ValidationError::InvalidChoose => code_msg(CODE_INVALID_CHOOSE, "invalid choose"),
        ValidationError::InvalidForEach => code_msg(CODE_INVALID_FOR_EACH, "invalid for_each"),
        ValidationError::InvalidTogether => code_msg(CODE_INVALID_TOGETHER, "invalid together"),
        ValidationError::InvalidCollect => code_msg(CODE_INVALID_COLLECT, "invalid collect"),
        ValidationError::InvalidReduce => code_msg(CODE_INVALID_REDUCE, "invalid reduce"),
        ValidationError::InvalidRepeat => code_msg(CODE_INVALID_REPEAT, "invalid repeat"),
        ValidationError::InvalidWait => code_msg(CODE_INVALID_WAIT, "invalid wait"),
        ValidationError::InvalidAsk => code_msg(CODE_INVALID_ASK, "invalid ask"),
        ValidationError::InvalidFinish => code_msg(CODE_INVALID_FINISH, "invalid finish"),
        ValidationError::InvalidRetry => code_msg(CODE_INVALID_RETRY, "invalid retry"),
        ValidationError::InvalidOnError => code_msg(CODE_INVALID_ON_ERROR, "invalid on_error"),
        _ => return None,
    };
    Some(parts)
}

fn limit_parts(error: &ValidationError) -> Option<(DiagnosticCode, String)> {
    let parts = match error {
        ValidationError::SecretResultLeak => {
            code_msg(CODE_SECRET_RESULT_LEAK, "secret result leak")
        }
        ValidationError::TypeMismatch { expected, found } => code_msg(
            CODE_TYPE_MISMATCH,
            format!("type mismatch: expected {expected}, found {found}"),
        ),
        ValidationError::PayloadTooLarge => code_msg(CODE_PAYLOAD_TOO_LARGE, "payload too large"),
        ValidationError::LimitRequired { resource } => {
            code_msg(CODE_LIMIT_REQUIRED, format!("limit required: {resource}"))
        }
        ValidationError::LimitExceeded { resource } => {
            code_msg(CODE_LIMIT_EXCEEDED, format!("limit exceeded: {resource}"))
        }
        ValidationError::UnsupportedTrigger { trigger } => code_msg(
            CODE_UNSUPPORTED_TRIGGER,
            format!("unsupported trigger: {trigger}"),
        ),
        ValidationError::HttpTriggerOutOfCore => {
            code_msg(CODE_HTTP_TRIGGER_OUT_OF_CORE, "HTTP trigger out of core")
        }
        _ => return None,
    };
    Some(parts)
}

fn gate_parts(error: &ValidationError) -> Option<(DiagnosticCode, String)> {
    let parts = match error {
        ValidationError::ExpressionStackExceeded { declared, limit } => code_msg(
            CODE_EXPRESSION_STACK_EXCEEDED,
            format!("expression stack exceeded: declared {declared}, limit {limit}"),
        ),
        ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
        } => code_msg(
            CODE_EXPRESSION_STACK_MISMATCH,
            format!(
                "expression stack mismatch: expr {expr_index}, declared {declared}, computed {computed}"
            ),
        ),
        ValidationError::AccessorSlotOutOfRange {
            accessor_index,
            slot,
            slot_count,
        } => code_msg(
            CODE_ACCESSOR_SLOT_OUT_OF_RANGE,
            format!(
                "accessor slot out of range: accessor {accessor_index}, slot {slot}, slot_count {slot_count}"
            ),
        ),
        ValidationError::AccessorPathInvalid {
            accessor_index,
            segment_index,
        } => code_msg(
            CODE_ACCESSOR_PATH_INVALID,
            format!("accessor path invalid: accessor {accessor_index}, segment {segment_index}"),
        ),
        ValidationError::AccessorPathTooDeep {
            accessor_index,
            depth,
            max,
        } => code_msg(
            CODE_ACCESSOR_PATH_TOO_DEEP,
            format!("accessor path too deep: accessor {accessor_index}, depth {depth}, max {max}"),
        ),
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index,
            segment_index,
            symbol,
            symbols_count,
        } => code_msg(
            CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS,
            format!(
                "accessor symbol out of bounds: accessor {accessor_index}, segment {segment_index}, symbol {symbol}, symbols_count {symbols_count}"
            ),
        ),
        ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
        } => code_msg(
            CODE_SLOT_REFERENCE_OUT_OF_RANGE,
            format!(
                "slot reference out of range: slot {slot}, slot_count {slot_count}, context {context}"
            ),
        ),
        _ => return None,
    };
    Some(parts)
}
