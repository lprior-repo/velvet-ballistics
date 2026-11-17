#[cfg(test)]
use super::*;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::CompiledNodeKind;

use super::super::test_helpers::{finish_node, make_parts, nop_node};

// -- Pass cases --

#[test]
fn accepts_valid_finish_node() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn accepts_valid_copy_node() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );
    let parts = make_parts(vec![node, finish_node(1, 0)], 2);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn accepts_valid_build_object() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildObject {
            fields: Box::new([
                (SymbolId::new(1), SlotIdx::new(0)),
                (SymbolId::new(2), SlotIdx::new(1)),
            ]),
        },
        Some(SlotIdx::new(2)),
        None,
    );
    let parts = make_parts(vec![node], 3);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn accepts_valid_build_list() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildList {
            items: Box::new([SlotIdx::new(0), SlotIdx::new(1)]),
        },
        Some(SlotIdx::new(2)),
        None,
    );
    let parts = make_parts(vec![node], 3);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn accepts_valid_set_const() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn accepts_expr_with_valid_slot_ref() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 2);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
        max_stack: 1,
    }]);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn accepts_nop_node() {
    let parts = make_parts(vec![nop_node(0)], 0);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

// -- Fail cases --

#[test]
fn rejects_output_slot_out_of_range() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(99)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn rejects_copy_source_out_of_range() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(50),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn rejects_expr_load_slot_out_of_range() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(99))]),
        max_stack: 1,
    }]);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn rejects_build_object_slot_out_of_range() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildObject {
            fields: Box::new([(SymbolId::new(1), SlotIdx::new(99))]),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn rejects_build_list_slot_out_of_range() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildList {
            items: Box::new([SlotIdx::new(50)]),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn rejects_do_input_out_of_range() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(1),
            input: SlotIdx::new(99),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn rejects_node_error_slot_out_of_range() {
    // Node-level error_slot is a SlotIdx reference and must be within slot_count.
    // Previously gate 09 only validated `node.output`, leaving `node.error_slot`
    // unchecked even though it is a slot reference written before routing to the
    // error handler.
    let node = vb_core::workflow::node_with_handler(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        None,
        None,
        StepIdx::new(0),
        Some(SlotIdx::new(99)),
    );
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn accepts_node_error_slot_in_range() {
    let node = vb_core::workflow::node_with_handler(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(1)),
        StepIdx::new(1),
        Some(SlotIdx::new(0)),
    );
    let parts = make_parts(vec![node, nop_node(1)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn accepts_node_error_slot_none() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(1)),
    );
    let parts = make_parts(vec![node, nop_node(1)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn checks_error_handler_owned_slots_in_both_gate_09_entry_points() {
    let cases: [(u16, Option<u16>); 6] = [
        (0, None),
        (0, Some(0)),
        (1, None),
        (1, Some(0)),
        (1, Some(1)),
        (1, Some(2)),
    ];
    for (slot_count, error_slot) in cases {
        let handler = vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: error_slot.map(SlotIdx::new).into(),
            },
            None,
            None,
        );
        let body = vb_core::workflow::node(
            StepIdx::new(1), CompiledNodeKind::Nop, None, None,
        );
        let target = vb_core::workflow::node(
            StepIdx::new(2), CompiledNodeKind::Nop, None, None,
        );
        let parts = make_parts(vec![handler, body, target], slot_count);
        let expected = match error_slot {
            Some(slot) if slot >= slot_count => Err(ValidationError::SlotReferenceOutOfRange {
                slot: usize::from(slot),
                slot_count: usize::from(slot_count),
                context: "node 0".to_owned(),
            }),
            _ => Ok(()),
        };
        assert_eq!(validate_gate_09_slot_references(&parts), expected);
        assert_eq!(crate::gates::validate_gate_09_slot_references(&parts), expected);
    }
}
