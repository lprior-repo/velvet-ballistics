use crate::gate_12_14_15::validate_gate_15_determinism_proof;
use crate::gates::validate_gate_15_determinism_proof as public_validate;
use crate::test_helpers::{copy_node, finish_node, make_parts, nop_node};
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};

fn node(index: u16, next: Option<u16>, ask: bool) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: Some(SlotIdx::new(0)),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind: if ask {
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            }
        } else {
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            }
        },
    }
}

fn validators() -> [fn(&WorkflowParts) -> crate::ValidationResult<()>; 3] {
    [
        public_validate,
        validate_gate_15_determinism_proof,
        super::validate_resume_targets,
    ]
}

fn assert_valid(parts: &WorkflowParts) {
    for v in validators() {
        assert_eq!(v(parts), Ok(()));
    }
}

fn assert_invalid(parts: &WorkflowParts, node_index: usize, detail: &str) {
    let expected = Err(crate::ValidationError::NodeKindConstraintViolation {
        node_index,
        detail: detail.to_string(),
    });
    for v in validators() {
        assert_eq!(v(parts), expected);
    }
}

#[test]
fn terminal_do_valid() {
    assert_valid(&make_parts(vec![node(0, None, false)], 1));
}

#[test]
fn terminal_ask_valid() {
    assert_valid(&make_parts(vec![node(0, None, true)], 1));
}

#[test]
fn adjacent_do_do_valid() {
    assert_valid(&make_parts(
        vec![node(0, Some(1), false), node(1, None, false)],
        2,
    ));
}

#[test]
fn adjacent_do_ask_valid() {
    assert_valid(&make_parts(
        vec![node(0, Some(1), false), node(1, None, true)],
        2,
    ));
}

#[test]
fn adjacent_ask_do_valid() {
    assert_valid(&make_parts(
        vec![node(0, Some(1), true), node(1, None, false)],
        2,
    ));
}

#[test]
fn adjacent_ask_ask_valid() {
    assert_valid(&make_parts(
        vec![node(0, Some(1), true), node(1, None, true)],
        2,
    ));
}

#[test]
fn three_consecutive_do_valid() {
    assert_valid(&make_parts(
        vec![
            node(0, Some(1), false),
            node(1, Some(2), false),
            node(2, None, false),
        ],
        3,
    ));
}

#[test]
fn skip_forward_valid() {
    assert_valid(&make_parts(
        vec![
            node(0, Some(2), false),
            node(1, None, false),
            node(2, None, false),
        ],
        3,
    ));
}

#[test]
fn oob_at_count_invalid() {
    for ask in [false, true] {
        assert_invalid(
            &make_parts(vec![node(0, Some(1), ask)], 1),
            0,
            "suspension resume target 1 must be forward and below node_count 1",
        );
    }
}

#[test]
fn oob_at_u16_max_invalid() {
    for ask in [false, true] {
        assert_invalid(
            &make_parts(vec![node(0, Some(u16::MAX), ask)], 1),
            0,
            &format!(
                "suspension resume target {} must be forward and below node_count 1",
                u16::MAX
            ),
        );
    }
}

#[test]
fn self_target_invalid() {
    for ask in [false, true] {
        assert_invalid(
            &make_parts(vec![node(0, Some(0), ask)], 1),
            0,
            "suspension resume target 0 must be forward and below node_count 1",
        );
    }
}

#[test]
fn backward_target_invalid() {
    for ask in [false, true] {
        assert_invalid(
            &make_parts(vec![node(0, None, false), node(1, Some(0), ask)], 2),
            1,
            "suspension resume target 0 must be forward and below node_count 2",
        );
    }
}

#[test]
fn deterministic_nodes_unaffected() {
    assert_valid(&make_parts(
        vec![nop_node(0), copy_node(1, 0, 1), finish_node(2, 0)],
        2,
    ));
}
