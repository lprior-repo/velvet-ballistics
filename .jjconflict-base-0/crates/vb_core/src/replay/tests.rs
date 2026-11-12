//! Tests for the replay module.

use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, ListId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::ConstValue;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ExprOp, ExprProgram, ResourceContract, SlotBranch,
    WorkflowParts, check_expr_stack_bound,
};

use crate::ids::ActionId;
use crate::value::Taint;

use super::step::ReplayAction;
use super::{ReplayEngine, ReplayError, SuspensionKind};

include!("tests/basic_replay.rs");

include!("tests/collect_pages.rs");

include!("tests/errors_and_build.rs");

include!("tests/choose_expr.rs");

include!("tests/choose_slot.rs");

include!("tests/determinism_and_boundaries.rs");
