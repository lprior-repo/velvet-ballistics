#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprProgram, ResourceContract, WorkflowParts,
};

pub(crate) fn workflow_parts_with_accessors(
    accessors: Box<[AccessorProgram]>,
    slot_count: u16,
    symbols_count: u32,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("kani_workflow"),
        digest: WorkflowDigest::from_bytes(kani::any()),
        nodes: bounded_nodes(),
        expressions: bounded_expressions(),
        accessors,
        constants: bounded_constants(),
        slot_count,
        symbols_count,
        entry: kani::any(),
        resource_contract: kani::any::<ResourceContract>(),
        step_names: bounded_step_names(),
    }
}

fn bounded_nodes() -> Box<[CompiledNode]> {
    if kani::any::<bool>() {
        let result: SlotIdx = kani::any();
        Box::new([CompiledNode {
            id: kani::any::<StepIdx>(),
            output: optional_slot(),
            next: optional_step(),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result },
        }])
    } else {
        Box::new([])
    }
}

fn optional_slot() -> Option<SlotIdx> {
    if kani::any::<bool>() {
        Some(kani::any::<SlotIdx>())
    } else {
        None
    }
}

fn optional_step() -> Option<StepIdx> {
    if kani::any::<bool>() {
        Some(kani::any::<StepIdx>())
    } else {
        None
    }
}

fn bounded_expressions() -> Box<[ExprProgram]> {
    if kani::any::<bool>() {
        Box::new([ExprProgram {
            ops: bounded_expr_ops(),
            max_stack: kani::any(),
        }])
    } else {
        Box::new([])
    }
}

fn bounded_expr_ops() -> Box<[vb_core::workflow::ExprOp]> {
    if kani::any::<bool>() {
        Box::new([kani::any::<vb_core::workflow::ExprOp>()])
    } else {
        Box::new([])
    }
}

fn bounded_constants() -> Box<[vb_core::value::ConstValue]> {
    if kani::any::<bool>() {
        Box::new([kani::any::<vb_core::value::ConstValue>()])
    } else {
        Box::new([])
    }
}

fn bounded_step_names() -> Box<[Box<str>]> {
    if kani::any::<bool>() {
        Box::new([Box::from("kani_step")])
    } else {
        Box::new([])
    }
}
