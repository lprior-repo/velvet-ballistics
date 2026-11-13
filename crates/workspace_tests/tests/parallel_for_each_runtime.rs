#![forbid(unsafe_code)]
//! End-to-end execution of whole-source parallel `for_each` through the runtime.

use vb_compile::compile_workflow;
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: parallel_runtime\nwhen: {manual: {}}\n";

fn source(at_once: &str) -> String {
    format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: 4}}\nsteps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      at_once: {at_once}\n      steps:\n        - id: leaf\n          set: {{output: picked, value: '$entry'}}\n  - id: done\n    finish: {{result: '$outer'}}\n"
    )
}

fn compile(at_once: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source(at_once).as_bytes()).map_err(|errors| format!("{errors:?}"))
}

fn drive_signal(
    plan: &CompiledWorkflow,
    items: &[i64],
    steps: u64,
) -> Result<RuntimeSignal, String> {
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), plan.node_count(), slot_count)
        .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(steps);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let input = plan.input_slots().first().ok_or("missing input slot")?.slot;
    let values: Vec<SlotValue> = items.iter().map(|value| SlotValue::I64(*value)).collect();
    let list = store
        .insert_list(values.into_boxed_slice())
        .map_err(|error| format!("{error}"))?;
    run.write_slot(input, SlotValue::List(list))
        .map_err(|error| format!("{error}"))?;
    drive_deterministic_full(
        plan,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &granted,
        1,
    )
    .map_err(|error| format!("{error}"))
}

/// F1 regression: lane work consumes the drive-step budget exactly like the
/// serial lowering, so a parallel `for_each` cannot run for free.
#[test]
fn parallel_for_each_exhausts_the_drive_step_budget() -> Result<(), String> {
    let parallel = compile("2")?;
    let serial = compile("1")?;
    assert_eq!(
        drive_signal(&parallel, &[1, 2, 3], 4)?,
        RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(
        drive_signal(&serial, &[1, 2, 3], 4)?,
        RuntimeSignal::StepBudgetExhausted
    );
    Ok(())
}

fn drive(plan: &CompiledWorkflow, items: &[i64]) -> Result<Vec<i64>, String> {
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), plan.node_count(), slot_count)
        .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(10_000);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();

    let input = plan
        .input_slots()
        .first()
        .ok_or("missing input slot")?
        .slot;
    let values: Vec<SlotValue> = items.iter().map(|value| SlotValue::I64(*value)).collect();
    let list = store
        .insert_list(values.into_boxed_slice())
        .map_err(|error| format!("{error}"))?;
    run.write_slot(input, SlotValue::List(list))
        .map_err(|error| format!("{error}"))?;

    let signal = drive_deterministic_full(
        plan,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &granted,
        1,
    )
    .map_err(|error| format!("{error}"))?;
    let RuntimeSignal::Finished(SlotValue::List(results)) = signal else {
        return Err(format!("unexpected signal: {signal:?}"));
    };
    let values = store
        .list(results)
        .map_err(|error| format!("{error}"))?;
    values
        .iter()
        .map(|value| match value {
            SlotValue::I64(number) => Ok(*number),
            other => Err(format!("unexpected item: {other:?}")),
        })
        .collect()
}

#[test]
fn parallel_for_each_returns_ordered_owned_results() -> Result<(), String> {
    let plan = compile("2")?;
    assert_eq!(drive(&plan, &[10, 20, 30])?, vec![10, 20, 30]);
    Ok(())
}

#[test]
fn at_once_one_and_raw_max_produce_identical_order() -> Result<(), String> {
    let serial = compile("1")?;
    let capped = compile("4294967295")?;
    assert_eq!(drive(&serial, &[4, 5, 6])?, vec![4, 5, 6]);
    assert_eq!(drive(&capped, &[4, 5, 6])?, vec![4, 5, 6]);
    Ok(())
}

#[test]
fn zero_item_parallel_for_each_finishes_with_empty_list() -> Result<(), String> {
    let plan = compile("2")?;
    assert_eq!(drive(&plan, &[])?, Vec::<i64>::new());
    Ok(())
}

#[test]
fn repeated_activation_on_fresh_frames_is_deterministic() -> Result<(), String> {
    let plan = compile("3")?;
    let first = drive(&plan, &[7, 8])?;
    let second = drive(&plan, &[7, 8])?;
    assert_eq!(first, vec![7, 8]);
    assert_eq!(second, first);
    Ok(())
}

fn replay(plan: &CompiledWorkflow, items: &[i64]) -> Result<Vec<i64>, String> {
    use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};

    let mut run = RunFrame::new(
        RunId::new(1),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let input = plan.input_slots().first().ok_or("missing input slot")?.slot;
    let values: Vec<SlotValue> = items.iter().map(|value| SlotValue::I64(*value)).collect();
    let list = store
        .insert_list(values.into_boxed_slice())
        .map_err(|error| format!("{error}"))?;
    run.write_slot(input, SlotValue::List(list))
        .map_err(|error| format!("{error}"))?;

    let mut states = ReplayCollectStates::new();
    let mut pc = StepIdx::new(0);
    for _ in 0..10_000 {
        let node = plan.node(pc).ok_or_else(|| format!("missing node {pc:?}"))?;
        match replay_step_with_collect(node, &mut run, &mut store, plan, &mut states)
            .map_err(|error| format!("{error:?}"))?
        {
            ReplayAction::Continue(next) => pc = next,
            ReplayAction::Finished => {
                let finish = (0..plan.node_count())
                    .filter_map(|index| plan.node(StepIdx::new(index)))
                    .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Finish { .. }))
                    .ok_or("missing finish")?;
                let vb_core::CompiledNodeKind::Finish { result } = finish.kind else {
                    return Err("finish kind".into());
                };
                let SlotValue::List(id) = *run.read_slot(result).map_err(|error| format!("{error}"))? else {
                    return Err("finish result is not a list".into());
                };
                return store
                    .list(id)
                    .map_err(|error| format!("{error:?}"))?
                    .iter()
                    .map(|value| match value {
                        SlotValue::I64(number) => Ok(*number),
                        other => Err(format!("unexpected replay item: {other:?}")),
                    })
                    .collect();
            }
            ReplayAction::Suspended { .. } => return Err("unexpected suspension".into()),
            _ => return Err("unknown replay action".into()),
        }
    }
    Err("replay step bound".into())
}

#[test]
fn replay_matches_runtime_for_parallel_for_each() -> Result<(), String> {
    let plan = compile("2")?;
    let runtime = drive(&plan, &[1, 2, 3])?;
    let replayed = replay(&plan, &[1, 2, 3])?;
    assert_eq!(runtime, vec![1, 2, 3]);
    assert_eq!(replayed, runtime);
    Ok(())
}

fn concurrent_ir(
    name: &str,
    nodes: Vec<vb_core::workflow::CompiledNode>,
    constants: Vec<vb_core::value::ConstValue>,
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    concurrent_ir_with_expressions(name, nodes, constants, Vec::new(), slot_count)
}

fn concurrent_ir_with_expressions(
    name: &str,
    nodes: Vec<vb_core::workflow::CompiledNode>,
    constants: Vec<vb_core::value::ConstValue>,
    expressions: Vec<vb_core::ExprProgram>,
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    use vb_core::ids::WorkflowDigest;
    use vb_core::workflow::{ResourceContract, WorkflowParts};

    let mut parts = WorkflowParts {
        name: Box::from(name),
        digest: WorkflowDigest::from_bytes([3u8; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    };
    let encoded = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&encoded).into());
    CompiledWorkflow::try_from_concurrent_parts(parts).map_err(|error| format!("{error:?}"))
}

fn drive_ir(plan: &CompiledWorkflow) -> Result<(RuntimeSignal, Vec<SlotValue>), String> {
    let mut run = RunFrame::new(
        RunId::new(1),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(10_000);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let signal = drive_deterministic_full(
        plan,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &granted,
        1,
    )
    .map_err(|error| format!("{error:?}"))?;
    let values = match &signal {
        RuntimeSignal::Finished(SlotValue::List(id)) => store
            .list(*id)
            .map_err(|error| format!("list: {error}"))?
            .to_vec(),
        _ => Vec::new(),
    };
    Ok((signal, values))
}

fn two_lane_nodes(first_success: bool) -> Vec<vb_core::workflow::CompiledNode> {
    use vb_core::workflow::{CompiledNodeKind as K, node};
    let n = |id: u16, kind: K, out: Option<u16>, next: Option<u16>| {
        node(
            StepIdx::new(id),
            kind,
            out.map(SlotIdx::new),
            next.map(StepIdx::new),
        )
    };
    let start = if first_success {
        K::FirstSuccessStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        }
    } else {
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        }
    };
    vec![
        n(0, start, None, None),
        n(1, K::SetConst { value: vb_core::ConstIdx::new(0) }, Some(1), Some(2)),
        n(2, K::ConcurrentLaneEnd { owner: StepIdx::new(0), result: Some(SlotIdx::new(1)) }, None, Some(5)),
        n(3, K::SetConst { value: vb_core::ConstIdx::new(1) }, Some(1), Some(4)),
        n(4, K::ConcurrentLaneEnd { owner: StepIdx::new(0), result: Some(SlotIdx::new(1)) }, None, Some(5)),
        n(5, K::ConcurrentGroupJoin { owner: StepIdx::new(0) }, Some(2), Some(6)),
        n(6, K::Finish { result: SlotIdx::new(2) }, None, None),
    ]
}

#[test]
fn together_concurrent_drains_all_lane_results() -> Result<(), String> {
    let plan = concurrent_ir(
        "together_drain",
        two_lane_nodes(false),
        vec![
            vb_core::value::ConstValue::I64(10),
            vb_core::value::ConstValue::I64(20),
        ],
        3,
    )?;
    let (signal, values) = drive_ir(&plan)?;
    assert!(matches!(signal, RuntimeSignal::Finished(_)));
    assert_eq!(values, vec![SlotValue::I64(10), SlotValue::I64(20)]);
    Ok(())
}

#[test]
fn first_success_returns_first_lane_value() -> Result<(), String> {
    let plan = concurrent_ir(
        "first_success",
        two_lane_nodes(true),
        vec![
            vb_core::value::ConstValue::I64(10),
            vb_core::value::ConstValue::I64(20),
        ],
        3,
    )?;
    let (signal, _) = drive_ir(&plan)?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(10)));
    Ok(())
}
#[path = "parallel_for_each_runtime/budget_resume.rs"]
mod budget_resume;
#[path = "parallel_for_each_runtime/first_success_expression.rs"]
mod first_success_expression;
#[path = "parallel_for_each_runtime/owned_reopen.rs"]
mod owned_reopen;
#[path = "parallel_for_each_runtime/fixed_group_resume.rs"]
mod fixed_group_resume;
