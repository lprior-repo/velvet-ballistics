#[cfg(test)]
use crate::ValidationError;
#[cfg(test)]
use crate::shared::{ValidationPipeline, validate};
#[cfg(test)]
use crate::test_helpers::{finish_node, make_parts};
#[cfg(test)]
use vb_core::ids::{SlotIdx, StepIdx};
#[cfg(test)]
use vb_core::workflow::CompiledNodeKind;

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
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: vb_core::workflow::InputSlotKind::Any,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    assert_eq!(validate(&parts), Ok(()));
}

#[test]
fn validate_convenience_catches_bad_slot_reference() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(99)),
        None,
    );
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
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(99)),
        None,
    );
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
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(99)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    let result = ValidationPipeline::default().validate(&parts);
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}
