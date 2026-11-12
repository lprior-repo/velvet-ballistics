// vb_core engine validate red-phase cycle detection tests
// Backward-edge and self-loop rejection coverage for the validator.

#![forbid(unsafe_code)]

use super::red_phase_behavior_tests::{nop_node_with_next, valid_parts_with_nodes};
use crate::engine::validate::validate_compiled_workflow;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNodeKind, ExprBranch, ExprOp, ExprProgram, WorkflowError, WorkflowParts,
};

mod cycle_detection_and_backward_edges {
    use super::*;
    use crate::workflow::node;

    #[test]
    fn rejects_direct_backward_edge_via_next() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                Some(StepIdx::new(0)),
            ),
        ]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_self_loop_via_next() {
        let parts = valid_parts_with_nodes(vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            Some(StepIdx::new(0)),
        )]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(0),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_jump_with_backward_target() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_choose_branch_with_backward_target() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(0),
                    }]
                    .into_boxed_slice(),
                    otherwise: None.into(),
                },
                None,
                None,
            ),
        ]);
        let p = WorkflowParts {
            semantic_source_identity: None,
            expressions: vec![ExprProgram {
                ops: Box::new([ExprOp::LoadConst(ConstIdx::new(0))]),
                max_stack: 1,
            }]
            .into_boxed_slice(),
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            ..parts
        };
        let result = validate_compiled_workflow(&p);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_backward_edge_with_exact_error_fields() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(0)),
            ),
        ]);
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            }
        );
    }
}
