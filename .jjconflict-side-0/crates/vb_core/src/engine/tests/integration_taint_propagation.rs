#![forbid(unsafe_code)]
//! Integration tests for taint propagation through EvalExpr, BuildObject,
//! BuildList, Choose, Finish, CopySlot, and resume_action_completion paths.
//!
//! These tests verify POST-001 through POST-008 and INV-001 through INV-007
//! from the vb-i94f contract.

use crate::errors::EngineError;
use crate::ids::{
    ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::value::{ConstValue, SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ExprProgram, ResourceContract,
    SlotBranch, WorkflowParts,
};

use super::ensure_equal;
use crate::action::{
    ActionContract, ActionName, ActionOutputReady, ActionTicket, Idempotency, RetrySafety,
    SideEffect, compute_action_idempotency_key,
};
use crate::engine::{EngineSignal, eval_expr_with_store, resume_action_completion, step_once};
use crate::frame::RunFrame;

// =============================================================================
// Test Infrastructure
// =============================================================================

fn action_ticket(run: RunId, step: StepIdx, seq: SeqNo, action: ActionId) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 1,
        fencing_token: 0,
    }
}

fn action_contract(action: ActionId, max_output_bytes: u32) -> ActionContract {
    ActionContract {
        id: action,
        name: ActionName::from_static_infallible("taint-action"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 128,
        max_output_bytes,
        timeout_ms: 1_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

fn action_output(output_slot: SlotIdx, value: SlotValue, taint: Taint) -> ActionOutputReady {
    ActionOutputReady {
        output_slot,
        value,
        taint,
        encoded_len: 2,
    }
}

include!("integration_taint_propagation/lattice_frame_and_eval_expr.rs");
include!("integration_taint_propagation/build_object_list_choose_finish.rs");
include!("integration_taint_propagation/copy_slot_resume_and_invariants.rs");
include!("integration_taint_propagation/error_cases_and_full_lattice.rs");
include!("integration_taint_propagation/escalation_and_random_time_dependent.rs");
include!("integration_taint_propagation/build_list_join_serialization_defaults.rs");
include!("integration_taint_propagation/output_invariant_nested_and_edge_cases.rs");
