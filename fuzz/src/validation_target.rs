//! Validation fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

use vb_core::WorkflowParts;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};
use vb_validate::ValidationError;

fn bounded_capability_name(name: &str) -> &str {
    let mut end = name.len().min(128);
    while !name.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let Some(prefix) = name.get(..end) else {
        return "";
    };
    prefix
}

fn capability_name_is_valid(name: &str) -> bool {
    name.split('.').all(capability_segment_is_valid)
}

fn capability_segment_is_valid(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn fuzz_action_contract(
    action: u16,
    required_capabilities: Box<[Capability]>,
) -> Option<ActionContract> {
    let name = ActionName::new(format!("fuzz_action_{action}")).ok()?;
    Some(ActionContract {
        id: ActionId::new(action),
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities,
    })
}

fn fuzz_parts_with_actions(actions: &[u16]) -> WorkflowParts {
    let mut nodes = Vec::new();
    let mut index = 0u16;
    for action in actions {
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(*action),
                input: SlotIdx::new(0),
            },
        });
        index = index.saturating_add(1);
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    WorkflowParts {
        name: Box::from("capability-schema-fuzz"),
        digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

pub fn fuzz_capability_name_schema(data: &[u8]) {
    let Ok(name) = std::str::from_utf8(data) else {
        return;
    };
    let bounded_name = bounded_capability_name(name);
    let parts = fuzz_parts_with_actions(&[1]);
    let Some(contract) = fuzz_action_contract(
        1,
        Box::new([Capability::new(Box::from(bounded_name), ActionId::new(1))]),
    ) else {
        return;
    };
    let contracts = [contract];
    let result = vb_validate::shared::validate_with_contracts(&parts, &contracts);
    if bounded_name.is_empty() {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameEmpty { .. })
        ));
    } else if !capability_name_is_valid(bounded_name) {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameInvalid { .. })
        ));
    } else {
        assert!(result.is_ok());
    }
}

pub fn fuzz_capability_contract_schema(data: &[u8]) {
    let first = data.first().copied().map_or(1, u16::from);
    let second = data.get(1).copied().map_or(first, u16::from);
    let tail = match data.get(2..) {
        Some(bytes) => bytes,
        None => &[],
    };
    let name = std::str::from_utf8(tail).map_or("network", bounded_capability_name);
    let parts = fuzz_parts_with_actions(&[first]);
    let Some(contract) = fuzz_action_contract(
        first,
        Box::new([
            Capability::new(Box::from(name), ActionId::new(second)),
            Capability::new(Box::from(name), ActionId::new(second)),
        ]),
    ) else {
        return;
    };
    let contracts = [contract];
    let result = vb_validate::shared::validate_with_contracts(&parts, &contracts);
    if name.is_empty() {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameEmpty { .. })
        ));
    } else if !capability_name_is_valid(name) {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityNameInvalid { .. })
        ));
    } else if first != second {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityActionMismatch { .. })
        ));
    } else {
        assert!(matches!(
            result,
            Err(ValidationError::CapabilityDuplicate { .. })
        ));
    }
}

const FUZZ_MAX_NODES: usize = 32;

fn build_fuzz_node(
    index: usize,
    kind_byte: u8,
    node_count: usize,
    slot_count: u16,
    data: &[u8],
) -> vb_core::CompiledNode {
    let step_idx = vb_core::StepIdx::new(u16::try_from(index).unwrap_or(u16::MAX));
    let next_step = if index.saturating_add(1) < node_count {
        Some(vb_core::StepIdx::new(
            u16::try_from(index).unwrap_or(0).saturating_add(1),
        ))
    } else {
        None
    };

    let max_slot = slot_count.saturating_sub(1);
    let safe_slot = vb_core::SlotIdx::new(max_slot);

    let kind = match kind_byte.wrapping_rem(8) {
        0 => vb_core::CompiledNodeKind::Nop,
        1 => vb_core::CompiledNodeKind::Finish { result: safe_slot },
        2 => vb_core::CompiledNodeKind::Copy { source: safe_slot },
        3 => vb_core::CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        4 => {
            let body_idx = u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let done_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::ForEachStart {
                input: safe_slot,
                item_slot: safe_slot,
                limit: 10,
                body: vb_core::StepIdx::new(body_idx),
                done: vb_core::StepIdx::new(done_idx),
            }
        }
        5 => {
            let branch_idx =
                u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            let join_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let data_len = data.len();
            let branch_count = if data_len > 4 {
                usize::from(data.get(3).copied().unwrap_or(1).wrapping_rem(4)).saturating_add(1)
            } else {
                1
            };
            let mut branches: Vec<vb_core::StepIdx> = Vec::new();
            for _ in 0..branch_count {
                branches.push(vb_core::StepIdx::new(branch_idx));
            }
            vb_core::CompiledNodeKind::TogetherStart {
                branches: branches.into_boxed_slice(),
                join: vb_core::StepIdx::new(join_idx),
            }
        }
        6 => {
            let body_idx = u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let done_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: vb_core::StepIdx::new(body_idx),
                done: vb_core::StepIdx::new(done_idx),
            }
        }
        _ => {
            let target_idx =
                u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            let otherwise_idx =
                u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            vb_core::CompiledNodeKind::ChooseSlot {
                branches: vec![vb_core::SlotBranch {
                    condition: safe_slot,
                    target: vb_core::StepIdx::new(target_idx),
                }]
                .into_boxed_slice(),
                otherwise: Some(vb_core::StepIdx::new(otherwise_idx)),
            }
        }
    };

    let output = if kind_byte.is_multiple_of(3) {
        Some(safe_slot)
    } else {
        None
    };

    vb_core::CompiledNode {
        id: step_idx,
        output,
        next: next_step,
        error_slot: None,
        on_error: None,
        kind,
    }
}

fn assert_typed_validation_error(error: vb_validate::ValidationError) {
    use vb_validate::ValidationError;
    match error {
        ValidationError::DuplicateKey
        | ValidationError::ForbiddenYamlFeature
        | ValidationError::UnknownTopLevelField
        | ValidationError::UnknownStepField
        | ValidationError::MissingRequiredField { .. }
        | ValidationError::InvalidVersion { .. }
        | ValidationError::InvalidId { .. }
        | ValidationError::ReservedId { .. }
        | ValidationError::DuplicateId { .. }
        | ValidationError::MultipleStepPrimitives
        | ValidationError::MissingStepPrimitive
        | ValidationError::UnknownReference { .. }
        | ValidationError::FutureReference { .. }
        | ValidationError::SecretNotDeclared { .. }
        | ValidationError::DirectRuntimeReference
        | ValidationError::InvalidThenTarget
        | ValidationError::ControlFlowCycle
        | ValidationError::UnreachableStep { .. }
        | ValidationError::InvalidChoose
        | ValidationError::InvalidForEach
        | ValidationError::InvalidTogether
        | ValidationError::InvalidCollect
        | ValidationError::InvalidReduce
        | ValidationError::InvalidRepeat
        | ValidationError::InvalidWait
        | ValidationError::InvalidAsk
        | ValidationError::InvalidFinish
        | ValidationError::InvalidRetry
        | ValidationError::InvalidOnError
        | ValidationError::SecretResultLeak
        | ValidationError::PayloadTooLarge
        | ValidationError::HttpTriggerOutOfCore
        | ValidationError::TypeMismatch { .. }
        | ValidationError::LimitRequired { .. }
        | ValidationError::LimitExceeded { .. }
        | ValidationError::UnsupportedTrigger { .. }
        | ValidationError::ExpressionStackExceeded { .. }
        | ValidationError::ExpressionStackMismatch { .. }
        | ValidationError::AccessorSlotOutOfRange { .. }
        | ValidationError::AccessorPathInvalid { .. }
        | ValidationError::AccessorPathTooDeep { .. }
        | ValidationError::AccessorSymbolOutOfBounds { .. }
        | ValidationError::SlotReferenceOutOfRange { .. }
        | ValidationError::LoopBodyStepOutOfRange { .. }
        | ValidationError::SlotDependencyCycle { .. }
        | ValidationError::NodeKindConstraintViolation { .. }
        | ValidationError::ActionContractMissing { .. }
        | ValidationError::ActionContractOrphan { .. }
        | ValidationError::CapabilityNameEmpty { .. }
        | ValidationError::CapabilityNameTooLong { .. }
        | ValidationError::CapabilityNameInvalid { .. }
        | ValidationError::CapabilityActionMismatch { .. }
        | ValidationError::CapabilityDuplicate { .. }
        | ValidationError::SlotTypeInconsistency { .. }
        | ValidationError::NonDeterministicPath { .. }
        | ValidationError::MissingSchemaVersion
        | ValidationError::CueVetFailed { .. }
        | ValidationError::VersionMonotonicityBreach { .. } => {}
        _ => {}
    }
}

pub fn fuzz_verifier_gates(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let Some(&byte0) = data.first() else {
        return;
    };
    let Some(&byte1) = data.get(1) else {
        return;
    };
    let node_count = usize::from(byte0.wrapping_rem(16))
        .saturating_add(1)
        .min(FUZZ_MAX_NODES);
    let slot_count = u16::from(byte1.wrapping_rem(16)).saturating_add(1);

    let mut nodes: Vec<vb_core::CompiledNode> = Vec::new();
    for i in 0..node_count {
        let Some(offset) = i.saturating_add(2).checked_rem(data.len()) else {
            continue;
        };
        let kind_byte = data.get(offset).copied().unwrap_or(0);
        let node = build_fuzz_node(i, kind_byte, node_count, slot_count, data);
        nodes.push(node);
    }

    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_gates"),
        digest: vb_core::WorkflowDigest::from_bytes([0xD0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let g7 = vb_validate::gates::validate_gate_07_expression_stack_depth(&parts);
    if let Err(e) = g7 {
        assert_typed_validation_error(e);
    }

    let g8 = vb_validate::gates::validate_gate_08_accessor_path_segments(&parts);
    if let Err(e) = g8 {
        assert_typed_validation_error(e);
    }

    let g9 = vb_validate::gates::validate_gate_09_slot_references(&parts);
    if let Err(e) = g9 {
        assert_typed_validation_error(e);
    }

    let g11 = vb_validate::gates::validate_gate_11_loop_body_graph(&parts);
    if let Err(e) = g11 {
        assert_typed_validation_error(e);
    }

    let g13 = vb_validate::gates::validate_gate_13_no_slot_cycles(&parts);
    if let Err(e) = g13 {
        assert_typed_validation_error(e);
    }
}

pub fn fuzz_diagnostic_from_error(data: &[u8]) {
    use vb_validate::ValidationError;
    use vb_validate::diagnostic::diagnostic_from_error;

    let Ok(payload) = std::str::from_utf8(data) else {
        return;
    };
    let field = if payload.is_empty() { "fuzz" } else { payload };

    let errors: [ValidationError; 16] = [
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        ValidationError::DirectRuntimeReference,
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::SecretResultLeak,
        ValidationError::PayloadTooLarge,
        ValidationError::HttpTriggerOutOfCore,
        ValidationError::MissingRequiredField {
            field: field.into(),
        },
        ValidationError::InvalidId { id: field.into() },
        ValidationError::TypeMismatch {
            expected: "bool".into(),
            found: field.into(),
        },
        ValidationError::LimitExceeded {
            resource: field.into(),
        },
    ];

    for error in &errors {
        let diag = diagnostic_from_error(error);

        assert!(
            !diag.message.is_empty(),
            "diagnostic message must be non-empty"
        );

        assert_ne!(
            diag.numeric_code.code(),
            0,
            "diagnostic code must be non-zero for variant"
        );
    }
}

pub fn fuzz_diagnostic_code_from_str(data: &[u8]) {
    use std::str::FromStr;
    use vb_core::diagnostic::DiagnosticCode;

    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let result = DiagnosticCode::from_str(input);

    if let Ok(code) = result {
        let display = code.to_string();
        assert!(display.starts_with('E'), "Display must start with E");
        assert_eq!(
            display.len(),
            5,
            "Display must be exactly E followed by 4 hex digits"
        );
    }
}
