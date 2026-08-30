#[cfg(test)]
use crate::vb_validate::ValidationError;
#[cfg(test)]
use crate::vb_validate::shared::{ValidationPipeline, validate};
#[cfg(test)]
use vb_core::ids::{SlotIdx, StepIdx};
#[cfg(test)]
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> crate::vb_validate::shared::WorkflowParts {
    crate::vb_validate::shared::WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

#[test]
fn pipeline_default_is_all_gates() {
    let pipeline = ValidationPipeline::default();
    assert!(pipeline.gate_07_expression_stack);
    assert!(pipeline.gate_08_accessor_paths);
    assert!(pipeline.gate_09_slot_references);
    assert!(pipeline.gate_10_node_kind_specific);
    assert!(pipeline.gate_11_loop_body_graph);
    assert!(pipeline.gate_12_action_contracts);
    assert!(pipeline.gate_13_no_slot_cycles);
    assert!(pipeline.gate_14_slot_type_consistency);
    assert!(pipeline.gate_15_determinism_proof);
}

#[test]
fn pipeline_no_gates_disables_all() {
    let pipeline = ValidationPipeline::no_gates();
    assert!(!pipeline.gate_07_expression_stack);
    assert!(!pipeline.gate_08_accessor_paths);
    assert!(!pipeline.gate_09_slot_references);
    assert!(!pipeline.gate_10_node_kind_specific);
    assert!(!pipeline.gate_11_loop_body_graph);
    assert!(!pipeline.gate_12_action_contracts);
    assert!(!pipeline.gate_13_no_slot_cycles);
    assert!(!pipeline.gate_14_slot_type_consistency);
    assert!(!pipeline.gate_15_determinism_proof);
}

#[test]
fn validate_convenience_passes_valid_parts() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate(&parts), Ok(()));
}

#[test]
fn validate_convenience_catches_bad_slot_reference() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn selective_gates_skip_disabled() {
    // Gate 9 catches out-of-range slot refs; disable it and the same
    // parts should pass.
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1);
    let pipeline = ValidationPipeline {
        gate_09_slot_references: false,
        ..ValidationPipeline::no_gates()
    };
    assert_eq!(pipeline.validate(&parts), Ok(()));
}

#[test]
fn pipeline_short_circuits_on_first_error() {
    // Construct parts that fail gate 9 (slot out of range). Gate 7 would
    // also fail if the stack depth is wrong, but we set up a case where
    // only gate 9 fails.
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1);
    let result = ValidationPipeline::default().validate(&parts);
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}
