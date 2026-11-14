use super::*;
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind as K, ExprOp, ExprProgram, ResourceContract, WorkflowParts, node,
};
use vb_core::{ConstIdx, CoreError, ExprIdx, SlotIdx, WorkflowDigest};
use vb_runtime::engine::RuntimeEngineError;

fn plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    slots: u16,
    concurrent: bool,
) -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::from("incremental_error_profiles"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count: slots,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    };
    let encoded = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&encoded).into());
    let compiled = if concurrent {
        CompiledWorkflow::try_from_concurrent_parts(parts)
    } else {
        CompiledWorkflow::try_from_parts(parts)
    };
    compiled.map_err(|error| format!("checked profile workflow: {error:?}"))
}

fn division() -> Result<ExprProgram, String> {
    ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ]))
    .map_err(|error| format!("checked division expression: {error:?}"))
}

fn failing_expression_plan() -> Result<CompiledWorkflow, String> {
    plan(
        vec![
            node(
                StepIdx::new(0),
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                K::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(10), ConstValue::I64(0)],
        vec![division()?],
        1,
        false,
    )
}

#[test]
fn division_by_zero_has_exact_runtime_and_replay_error_variants() -> Result<(), String> {
    let workflow = failing_expression_plan()?;
    let mut run = RunFrame::new(
        RunId::new(41_704),
        StepIdx::new(0),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| format!("runtime frame: {error}"))?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(16);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let runtime_result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &granted,
        1,
    );
    assert_eq!(
        runtime_result,
        Err(RuntimeEngineError::Core(CoreError::DivisionByZero))
    );
    let mut replay_run = RunFrame::new(
        RunId::new(41_706),
        StepIdx::new(0),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| format!("replay frame: {error}"))?;
    let mut replay_store = ValueStore::new();
    let mut replay_states = ReplayCollectStates::new();
    let replay_node = workflow
        .node(StepIdx::new(0))
        .ok_or("missing replay EvalExpr")?;
    match replay_step_with_collect(
        replay_node,
        &mut replay_run,
        &mut replay_store,
        &workflow,
        &mut replay_states,
    ) {
        Err(vb_core::replay::ReplayError::ExpressionEvalFailed { step }) => {
            assert_eq!(step, StepIdx::new(0));
        }
        Err(error) => return Err(format!("unexpected typed replay error: {error:?}")),
        Ok(_) => return Err("division by zero unexpectedly replayed successfully".to_owned()),
    }
    Ok(())
}

#[path = "profiles/fatal.rs"]
mod fatal;
#[path = "profiles/first_success.rs"]
mod first_success;
