//! Verifier-gate fuzz target body.

use vb_validate::ValidationError;

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
        4 => foreach_start(index, node_count, safe_slot),
        5 => together_start(index, node_count, data),
        6 => repeat_start(index, node_count),
        _ => choose_slot(index, node_count, safe_slot),
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

fn bounded_next(index: usize, node_count: usize, add: usize) -> vb_core::StepIdx {
    let idx = u16::try_from(index.saturating_add(add).min(node_count.saturating_sub(1)))
        .unwrap_or(0);
    vb_core::StepIdx::new(idx)
}

fn foreach_start(
    index: usize,
    node_count: usize,
    safe_slot: vb_core::SlotIdx,
) -> vb_core::CompiledNodeKind {
    vb_core::CompiledNodeKind::ForEachStart {
        input: safe_slot,
        item_slot: safe_slot,
        limit: 10,
        body: bounded_next(index, node_count, 1),
        done: bounded_next(index, node_count, 2),
    }
}

fn together_start(index: usize, node_count: usize, data: &[u8]) -> vb_core::CompiledNodeKind {
    let branch_idx = bounded_next(index, node_count, 1);
    let join = bounded_next(index, node_count, 2);
    let branch_count = if data.len() > 4 {
        usize::from(data.get(3).copied().unwrap_or(1).wrapping_rem(4)).saturating_add(1)
    } else {
        1
    };
    let mut branches: Vec<vb_core::StepIdx> = Vec::new();
    for _ in 0..branch_count {
        branches.push(branch_idx);
    }
    vb_core::CompiledNodeKind::TogetherStart {
        branches: branches.into_boxed_slice(),
        join,
    }
}

fn repeat_start(index: usize, node_count: usize) -> vb_core::CompiledNodeKind {
    vb_core::CompiledNodeKind::RepeatStart {
        max_attempts: 3,
        body: bounded_next(index, node_count, 1),
        done: bounded_next(index, node_count, 2),
    }
}

fn choose_slot(
    index: usize,
    node_count: usize,
    safe_slot: vb_core::SlotIdx,
) -> vb_core::CompiledNodeKind {
    vb_core::CompiledNodeKind::ChooseSlot {
        branches: vec![vb_core::SlotBranch {
            condition: safe_slot,
            target: bounded_next(index, node_count, 1),
        }]
        .into_boxed_slice(),
        otherwise: Some(bounded_next(index, node_count, 2)),
    }
}

fn assert_typed_validation_error(error: vb_validate::ValidationError) {
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
        nodes.push(build_fuzz_node(i, kind_byte, node_count, slot_count, data));
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
    for result in [
        vb_validate::gates::validate_gate_07_expression_stack_depth(&parts),
        vb_validate::gates::validate_gate_08_accessor_path_segments(&parts),
        vb_validate::gates::validate_gate_09_slot_references(&parts),
        vb_validate::gates::validate_gate_11_loop_body_graph(&parts),
        vb_validate::gates::validate_gate_13_no_slot_cycles(&parts),
    ] {
        if let Err(e) = result {
            assert_typed_validation_error(e);
        }
    }
}
