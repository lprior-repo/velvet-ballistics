use crate::budget::{BudgetDimension, BudgetError};
use crate::ids::{
    AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE, MAX_PATH_DEPTH};
use crate::value::ConstValue;
use crate::workflow::{
    AccessorProgram, CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind,
    CompiledWorkflow, ContractResource, CoreError, ErrorRouting, ExprBranch, ExprOp, ExprProgram,
    InputConstraintIssue, InputSlotKind, NodeContinuation, NodeOutput, PathSegment,
    ResourceContract, SlotBranch, WorkflowError, WorkflowParts, check_expr_stack_bound,
    validate_budget_result,
};
use std::fmt::Debug;

fn external_input(kind: InputSlotKind) -> Box<[CompiledInputSlot]> {
    Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind,
        constraint: CompiledInputConstraint::required(),
    }])
}

include!("tests/expr_and_parts.rs");

include!("tests/slots_and_errors.rs");

include!("tests/phase46_a.rs");

include!("tests/phase46_b.rs");

include!("tests/node_kinds.rs");

include!("tests/budgets_and_proptests.rs");

include!("tests/construction_and_constraints.rs");
