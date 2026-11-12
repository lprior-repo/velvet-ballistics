use super::support::{Case, TestResult, compile};
use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::{ObjectField, ValueStoreCheckpointV1};
use vb_core::workflow::{CompiledNodeKind, ExprOp};
use vb_core::{
    CompiledWorkflow, ConstIdx, ConstValue, ExprIdx, InputSlotKind, ObjectId, RunId, SlotIdx,
    SlotValue, StepIdx, SymbolId, Taint, ValueStore,
};
use vb_runtime::runtime::Runtime;

pub(super) const SOURCE: &str = "version: velvet-ballistics/v1\nname: bound_symbol_public_has\nwhen:\n  manual: {}\ninputs:\n  obj: { is: object }\nsteps:\n  - id: before\n    set: { output: before_value, value: '0' }\n  - id: check\n    set: { output: result, value: 'has($input.obj, \"key\")' }\n  - id: done\n    finish: { result: result }\n";
pub(super) const COLD_SOURCE: &str = "version: velvet-ballistics/v1\nname: bound_symbol_public_cold\nwhen:\n  manual: {}\ninputs:\n  obj: { is: object }\n  token: { is: text }\nsteps:\n  - id: before\n    set: { output: before_value, value: '$input.token' }\n  - id: check\n    set: { output: result, value: 'has($input.obj, \"key\")' }\n  - id: done\n    finish: { result: result }\n";

pub(super) fn fresh_case(key: &str) -> TestResult<Case> {
    let workflow = compile(SOURCE)?;
    assert_fresh_layout(&workflow)?;
    persist(
        workflow,
        SOURCE,
        key,
        Box::from([(
            SlotIdx::new(2),
            SlotValue::Object(ObjectId::new(0)),
            Taint::Clean,
        )]),
    )
}

pub(super) fn cold_case(key: &str) -> TestResult<Case> {
    let workflow = compile(COLD_SOURCE)?;
    assert_cold_layout(&workflow)?;
    persist(
        workflow,
        COLD_SOURCE,
        key,
        Box::from([
            (
                SlotIdx::new(2),
                SlotValue::Object(ObjectId::new(0)),
                Taint::Clean,
            ),
            (
                SlotIdx::new(3),
                SlotValue::Symbol(SymbolId::new(0)),
                Taint::Clean,
            ),
        ]),
    )
}

fn persist(
    workflow: CompiledWorkflow,
    source: &str,
    key: &str,
    roots: Box<[(SlotIdx, SlotValue, Taint)]>,
) -> TestResult<Case> {
    assert_eq!(workflow.resource_contract().max_slots, 1024);
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    assert_eq!(store.insert_symbol("noise")?, SymbolId::new(0));
    assert_eq!(store.insert_symbol(key)?, SymbolId::new(1));
    let object = store.insert_object(Box::from([ObjectField::clean(
        SymbolId::new(1),
        SlotValue::Symbol(SymbolId::new(0)),
    )]))?;
    assert_eq!(object, ObjectId::new(0));
    let bundle = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow)?;
    Case::persist(workflow, source, bundle, RunId::new(17), &[])
}

pub(super) fn assert_fresh_layout(workflow: &CompiledWorkflow) -> TestResult {
    let parts = workflow.to_parts();
    assert_eq!(parts.slot_count, 3);
    assert_eq!(
        parts.constants.as_ref(),
        &[ConstValue::I64(0), ConstValue::Symbol(SymbolId::new(0))]
    );
    assert_eq!(parts.input_slots.len(), 1);
    let input = parts.input_slots.first().ok_or("missing Object input")?;
    assert_eq!(
        (input.slot, input.kind),
        (SlotIdx::new(2), InputSlotKind::Object)
    );
    assert_eq!(
        &workflow.node(StepIdx::new(0)).ok_or("before missing")?.kind,
        &CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }
    );
    assert_has(workflow, ExprIdx::new(0), ConstIdx::new(1))?;
    assert_nodes(workflow, ExprIdx::new(0))
}

pub(super) fn assert_cold_layout(workflow: &CompiledWorkflow) -> TestResult {
    let parts = workflow.to_parts();
    assert_eq!(parts.slot_count, 4);
    assert_eq!(
        parts.constants.as_ref(),
        &[ConstValue::Symbol(SymbolId::new(0))]
    );
    assert_cold_inputs(workflow);
    assert_eq!(
        &workflow.node(StepIdx::new(0)).ok_or("before missing")?.kind,
        &CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0)
        }
    );
    assert_eq!(
        workflow
            .expression(ExprIdx::new(0))
            .ok_or("copy expression missing")?
            .ops
            .as_ref(),
        &[ExprOp::LoadSlot(SlotIdx::new(3))]
    );
    assert_has(workflow, ExprIdx::new(1), ConstIdx::new(0))?;
    assert_nodes(workflow, ExprIdx::new(1))
}

fn assert_cold_inputs(workflow: &CompiledWorkflow) {
    let parts = workflow.to_parts();
    let inputs: Vec<_> = parts
        .input_slots
        .iter()
        .map(|input| (input.slot, input.kind))
        .collect();
    assert_eq!(
        inputs,
        [
            (SlotIdx::new(2), InputSlotKind::Object),
            (SlotIdx::new(3), InputSlotKind::Symbol)
        ]
    );
}

fn assert_has(workflow: &CompiledWorkflow, expr: ExprIdx, constant: ConstIdx) -> TestResult {
    assert_eq!(
        workflow.symbol_payloads(),
        Some([Box::<str>::from("key")].as_slice())
    );
    assert_eq!(workflow.symbols_count(), 1);
    assert_eq!(
        workflow
            .expression(expr)
            .ok_or("Has expression missing")?
            .ops
            .as_ref(),
        &[
            ExprOp::LoadSlot(SlotIdx::new(2)),
            ExprOp::LoadConst(constant),
            ExprOp::Has,
        ]
    );
    Ok(())
}

fn assert_nodes(workflow: &CompiledWorkflow, expr: ExprIdx) -> TestResult {
    assert_eq!(workflow.entry(), StepIdx::new(0));
    assert_eq!(workflow.node_count(), 3);
    let before = workflow.node(StepIdx::new(0)).ok_or("before missing")?;
    assert_eq!(
        (before.output.slot(), before.routing.next.step()),
        (Some(SlotIdx::new(0)), Some(StepIdx::new(1)))
    );
    assert_eq!(workflow.node(StepIdx::new(1)), Some(&has_node(expr)));
    assert_eq!(workflow.node(StepIdx::new(2)), Some(&finish_node()));
    Ok(())
}

fn has_node(expr: ExprIdx) -> vb_core::CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::EvalExpr { expr },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    )
}

fn finish_node() -> vb_core::CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    )
}

pub(super) fn store(case: &Case, symbols: &[&str], key: u32) -> ValueStoreCheckpointV1 {
    let mut expected = case.original.checkpoint().clone();
    expected.symbols = symbols
        .iter()
        .map(|value| Box::<str>::from(*value))
        .collect();
    expected.objects = vec![Box::from([ObjectField::clean(
        SymbolId::new(key),
        SlotValue::Symbol(SymbolId::new(1)),
    )])];
    expected
}

pub(super) fn frame(
    case: &Case,
    pc: u16,
    executed: u64,
    slots: Vec<Option<SlotValue>>,
) -> RunFrameCheckpointV1 {
    RunFrameCheckpointV1 {
        run_id: case.run,
        pc: StepIdx::new(pc),
        executed,
        max_parallel_in_flight: 0,
        parallel_in_flight: 0,
        states: step_states(pc),
        taint: vec![Taint::Clean; slots.len()],
        slots,
    }
}

pub(super) fn assert_state(
    case: &Case,
    actor: &Runtime,
    expected_store: &ValueStoreCheckpointV1,
    expected_frame: RunFrameCheckpointV1,
) -> TestResult {
    let actual = actor.inspect_run_state(case.run)?;
    assert_eq!(actual.frame.checkpoint_v1(), expected_frame);
    assert_eq!(&actual.store.checkpoint_v1(), expected_store);
    assert_eq!(actual.workflow, case.workflow);
    assert_eq!(actual.run_attempt.get(), 1);
    assert_eq!(actual.action_contracts, case.artifact.action_contracts);
    assert_eq!(actual.action_attempts.as_ref(), &[0; 3]);
    let admission = actual
        .admission
        .as_ref()
        .ok_or("missing historical admission")?;
    assert_eq!(admission.artifact_digest(), case.artifact.digest);
    assert_eq!(
        admission.policy_pin(),
        vb_storage::RunPolicyPin::Pinned(case.artifact.policy_digest)
    );
    Ok(())
}

fn step_states(pc: u16) -> Vec<CheckpointStepState> {
    if pc == 1 {
        vec![
            CheckpointStepState::Succeeded,
            CheckpointStepState::Pending,
            CheckpointStepState::Pending,
        ]
    } else {
        vec![
            CheckpointStepState::Succeeded,
            CheckpointStepState::Succeeded,
            CheckpointStepState::Pending,
        ]
    }
}
