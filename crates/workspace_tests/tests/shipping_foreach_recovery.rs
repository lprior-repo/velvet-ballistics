#![forbid(unsafe_code)]
//! Public live/persist/drop/reopen/recover/replay parity for owned ForEach.
//!
//! Bead: vb-eirmk.7.1. Every scenario starts from a real owned submission on a
//! strict Fjall journal, drives the live runtime partially, drops the runtime
//! and journal handles, reopens the same database, recovers through the public
//! runtime boundary, and compares the replayed execution with the live run.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, Idempotency,
    RetrySafety, SideEffect,
};
use vb_core::frame::RunFrameCheckpointV1;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::join_taint;
use vb_core::value_store::{ObjectField, ValueStoreCheckpointV1};
use vb_core::{
    ActionId, CompiledNodeKind as K, CompiledWorkflow, ListId, RunId, RuntimePolicy, SlotIdx,
    SlotValue, StepIdx, StepState, Taint, ValueStore, WorkflowDigest,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, PendingTimerKind, ShardConfig};
use vb_storage::recovery::replay_owned_for_each;
use vb_storage::{
    DecodedSlotWrittenExtra, FjallJournal, JournalEvent, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord, decode_slot_written_extra,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const RUN: RunId = RunId::new(7_101);

fn runtime(journal: Arc<FjallJournal>, step_budget_per_tick: u64) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

fn multi_source(result: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: shipping_foreach_recovery\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: any, max: 4}}}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      at_once: 2\n      steps:\n        - id: projected\n          set: {{output: projected, value: '$entry'}}\n  - id: check\n    set: {{output: check, value: '{result}'}}\n  - id: done\n    finish: {{result: '$check'}}\n"
    )
}

fn nested_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_nested_foreach\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: outer\n    for_each:\n      variable: row\n      input: $input.items\n      steps:\n        - id: inner\n          for_each:\n            variable: cell\n            input: '$row'\n            steps:\n              - id: projected\n                set: {output: projected, value: '$cell'}\n  - id: check\n    set: {output: check, value: 'count($step.outer) == 2 and count($step.outer.0) == 2 and $step.outer.0.0 == 4 and $step.outer.0.1 == 1 and count($step.outer.1) == 1 and $step.outer.1.0 == 9'}\n  - id: done\n    finish: {result: '$check'}\n"
        .to_owned()
}

fn reduce_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_reduce_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: folded\n    reduce:\n      variable: total\n      input: $input.items\n      initial: '10'\n      steps:\n        - id: update\n          set: {output: local, value: '$total + 1'}\n  - id: done\n    finish: {result: '$steps.folded'}\n"
        .to_owned()
}

fn together_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_together_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: group\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set: {output: left, value: 'count($input.items)'}\n        - label: right\n          steps:\n            - id: right_value\n              set: {output: right, value: '7'}\n  - id: check\n    set: {output: check, value: '$step.group.0 + $step.group.1'}\n  - id: done\n    finish: {result: '$check'}\n"
        .to_owned()
}

fn repeat_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_repeat_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: projected\n          set: {output: projected, value: 'count($input.items)'}\n  - id: check\n    set: {output: check, value: '$step.loop'}\n  - id: done\n    finish: {result: '$check'}\n"
        .to_owned()
}

fn object_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_object_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n"
        .to_owned()
}

fn ask_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_ask_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: asked\n          ask:\n            prompt: '$entry'\n  - id: done\n    finish: {result: '$step.loop'}\n"
        .to_owned()
}

fn ask_timeout_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_ask_timeout_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: asked\n          ask: {prompt: '$entry', timeout: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n"
        .to_owned()
}

fn action_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_action_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: work\n          do: {action: '0', input: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n"
        .to_owned()
}

fn wait_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_wait_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: waited\n          wait: {timeout: '$entry'}\n        - id: observed\n          set: {output: observed, value: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n"
        .to_owned()
}

fn action_contracts() -> TestResult<Box<[ActionContract]>> {
    let name = ActionName::new("shipping-action")?;
    Ok(Box::from([ActionContract {
        id: ActionId::new(0),
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }]))
}

fn action_output(output: SlotIdx, value: SlotValue) -> TestResult<ActionOutputReady> {
    let encoded = postcard::to_allocvec(&value)?;
    Ok(ActionOutputReady {
        output_slot: output,
        value,
        taint: Taint::Clean,
        encoded_len: u32::try_from(encoded.len())?,
    })
}

struct Cut {
    directory: tempfile::TempDir,
    workflow: CompiledWorkflow,
    roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    input_store: ValueStoreCheckpointV1,
    live_frame: RunFrameCheckpointV1,
    live_store: ValueStoreCheckpointV1,
    events: Vec<JournalEvent>,
}

fn step_with(workflow: &CompiledWorkflow, predicate: impl Fn(&K) -> bool) -> TestResult<StepIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| predicate(&node.kind))
        .map(|node| node.id)
        .ok_or_else(|| "step not found".into())
}

fn cut_with(
    source: &str,
    stop: (StepIdx, StepState),
    build: impl FnOnce(&mut ValueStore) -> TestResult<(ListId, Taint)>,
) -> TestResult<Cut> {
    cut_with_nth(source, stop, 1, build)
}

fn cut_with_nth(
    source: &str,
    stop: (StepIdx, StepState),
    occurrences: usize,
    build: impl FnOnce(&mut ValueStore) -> TestResult<(ListId, Taint)>,
) -> TestResult<Cut> {
    cut_with_actions_nth(source, stop, occurrences, &[], build)
}

fn cut_with_actions_nth(
    source: &str,
    stop: (StepIdx, StepState),
    occurrences: usize,
    contracts: &[ActionContract],
    build: impl FnOnce(&mut ValueStore) -> TestResult<(ListId, Taint)>,
) -> TestResult<Cut> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let (list, root_taint) = build(&mut store)?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing input slot")?
        .slot;
    let roots = Box::from([(input, SlotValue::List(list), root_taint)]);
    let bundle = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow)?;
    let input_store = bundle.checkpoint().clone();
    let roots = bundle.roots().to_vec().into_boxed_slice();
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        contracts,
    )?;
    let source_record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let header = RunHeaderRecord {
        run: RUN,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    };
    let submission = RunSubmission::owned(source_record, artifact, inputs, header)?;
    let directory = tempfile::tempdir()?;
    let (live_frame, live_store, events) = {
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        let mut runtime = runtime(journal.clone(), 1);
        runtime.activate_owned_submission(RUN, source_digest)?;
        let mut live = None;
        let mut seen = 0_usize;
        for _ in 0..256 {
            runtime.tick_all()?;
            let state = runtime
                .inspect_run_state(RUN)
                .map_err(|error| format!("live inspect: {error}"))?;
            if state.frame.step_state(stop.0)? == stop.1 {
                seen = seen.saturating_add(1);
                if seen >= occurrences {
                    live = Some((state.frame.checkpoint_v1(), state.store.checkpoint_v1()));
                    break;
                }
            }
        }
        let (live_frame, live_store) = live.ok_or("cut step state never reached")?;
        (live_frame, live_store, journal.events_for_run_full(RUN)?)
    };
    Ok(Cut {
        directory,
        workflow,
        roots,
        input_store,
        live_frame,
        live_store,
        events,
    })
}

fn cut_at(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
    stop: (StepIdx, StepState),
) -> TestResult<Cut> {
    cut_at_nth(source, values, taints, stop, 1)
}

fn cut_at_nth(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
    stop: (StepIdx, StepState),
    occurrences: usize,
) -> TestResult<Cut> {
    cut_with_nth(source, stop, occurrences, |store| {
        let root_taint = taints
            .iter()
            .fold(Taint::Clean, |joined, taint| join_taint(joined, *taint));
        let list = store.insert_list_with_taint(
            values.to_vec().into_boxed_slice(),
            taints.to_vec().into_boxed_slice(),
        )?;
        Ok((list, root_taint))
    })
}

fn cut_at_actions(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
    stop: (StepIdx, StepState),
    contracts: &[ActionContract],
) -> TestResult<Cut> {
    cut_with_actions_nth(source, stop, 1, contracts, |store| {
        let root_taint = taints
            .iter()
            .fold(Taint::Clean, |joined, taint| join_taint(joined, *taint));
        let list = store.insert_list_with_taint(
            values.to_vec().into_boxed_slice(),
            taints.to_vec().into_boxed_slice(),
        )?;
        Ok((list, root_taint))
    })
}

fn replay_parity(cut: &Cut) -> TestResult<vb_storage::recovery::OwnedForEachReplay> {
    let replay = replay_owned_for_each(
        &cut.events,
        &cut.workflow,
        &cut.roots,
        Some(&cut.input_store),
    )?
    .ok_or("owned ForEach replay not selected")?;
    assert_eq!(replay.frame.checkpoint_v1(), cut.live_frame);
    assert_eq!(replay.store, cut.live_store);
    Ok(replay)
}

fn recover_and_finish(cut: &Cut) -> TestResult<Vec<JournalEvent>> {
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    let product = runtime
        .recover_product(RUN)
        .map_err(|error| format!("recover_product: {error:?}"))?;
    match &product {
        RuntimeRecoveryProduct::Resumable(_) => {}
        other => return Err(format!("owned ForEach recovery is not resumable: {other:?}").into()),
    }
    runtime
        .recover_and_resume(RUN)
        .map_err(|error| format!("recover_and_resume: {error:?}"))?;
    for _ in 0..256 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    Ok(journal.events_for_run_full(RUN)?)
}

fn terminal_result(events: &[JournalEvent]) -> TestResult<(SlotValue, Taint)> {
    let result = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("missing RunFinished")?;
    let (value, extra) = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot,
                value: Some(value),
                extra,
                ..
            } if *slot == result => Some((value, extra)),
            _ => None,
        })
        .ok_or("missing terminal slot write")?;
    let value = postcard::from_bytes(value)?;
    let extra = extra.as_deref().ok_or("missing taint extra")?;
    let taint = match decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))? {
        DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
        DecodedSlotWrittenExtra::LegacyFrameExtra(_) => return Err("legacy taint extra".into()),
    };
    Ok((value, taint))
}

#[test]
fn strict_owned_foreach_partial_run_recovers_and_resumes_after_reopen() -> TestResult {
    let source = multi_source("$step.loop.0 * 100 + $step.loop.1 * 10 + $step.loop.2");
    let cut = cut_at(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Secret, Taint::Clean, Taint::DerivedFromSecret],
        (body_step(&source)?, StepState::Succeeded),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(312), Taint::Secret),
        "ordered materialization and taint must survive recovery"
    );
    Ok(())
}

fn body_step(source: &str) -> TestResult<StepIdx> {
    control_step(source, |kind| matches!(kind, K::ForEachNext { .. }))
}

fn control_step(source: &str, predicate: impl Fn(&K) -> bool) -> TestResult<StepIdx> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    step_with(&workflow, predicate)
}

#[test]
fn empty_and_single_owned_foreach_sources_recover_after_reopen() -> TestResult {
    for (values, taints, result, expected, taint) in [
        (
            Vec::new(),
            Vec::new(),
            "count($step.loop)",
            SlotValue::I64(0),
            Taint::Clean,
        ),
        (
            vec![SlotValue::I64(7)],
            vec![Taint::Secret],
            "$step.loop.0",
            SlotValue::I64(7),
            Taint::Secret,
        ),
    ] {
        let source = multi_source(result);
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let stop = if values.is_empty() {
            (
                step_with(&workflow, |kind| matches!(kind, K::ForEachStart { .. }))?,
                StepState::Succeeded,
            )
        } else {
            (
                step_with(&workflow, |kind| matches!(kind, K::ForEachNext { .. }))?,
                StepState::Succeeded,
            )
        };
        let cut = cut_at(&source, &values, &taints, stop)?;
        replay_parity(&cut)?;
        let events = recover_and_finish(&cut)?;
        assert_eq!(terminal_result(&events)?, (expected, taint));
    }
    Ok(())
}

#[test]
fn nested_owned_foreach_recovers_outer_inner_activation_order() -> TestResult {
    let source = nested_source();
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let inner_body = step_with(&workflow, |kind| matches!(kind, K::EvalExpr { .. }))?;
    let cut = cut_with(&source, (inner_body, StepState::Succeeded), |store| {
        let first = store.insert_list_with_taint(
            Box::from([SlotValue::I64(4), SlotValue::I64(1)]),
            Box::from([Taint::Secret, Taint::Secret]),
        )?;
        let second = store
            .insert_list_with_taint(Box::from([SlotValue::I64(9)]), Box::from([Taint::Secret]))?;
        let outer = store.insert_list_with_taint(
            Box::from([SlotValue::List(first), SlotValue::List(second)]),
            Box::from([Taint::Secret, Taint::Secret]),
        )?;
        Ok((outer, Taint::Secret))
    })?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::Bool(true), Taint::Secret)
    );
    Ok(())
}

#[test]
fn repeated_owned_foreach_activation_rearms_body_steps() -> TestResult {
    let source = multi_source("count($step.loop)");
    let cut = cut_at_nth(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Secret, Taint::Clean, Taint::Secret],
        (body_step(&source)?, StepState::Succeeded),
        2,
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(3), Taint::Secret)
    );
    Ok(())
}

#[test]
fn owned_reduce_recovers_after_reopen() -> TestResult {
    let source = reduce_source();
    let cut = cut_at(
        &source,
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(1)],
        &[Taint::Secret, Taint::Clean, Taint::Clean],
        (
            control_step(&source, |kind| matches!(kind, K::ReduceFinish { .. }))?,
            StepState::Succeeded,
        ),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(13), Taint::Clean)
    );
    Ok(())
}

#[test]
fn owned_together_recovers_after_reopen() -> TestResult {
    let source = together_source();
    let cut = cut_at(
        &source,
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(1)],
        &[Taint::Clean, Taint::Clean, Taint::Clean],
        (
            control_step(&source, |kind| matches!(kind, K::TogetherJoin { .. }))?,
            StepState::Succeeded,
        ),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(10), Taint::Clean)
    );
    Ok(())
}

#[test]
fn owned_repeat_recovers_after_reopen() -> TestResult {
    let source = repeat_source();
    let cut = cut_at(
        &source,
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(1)],
        &[Taint::Clean, Taint::Clean, Taint::Clean],
        (
            control_step(&source, |kind| matches!(kind, K::RepeatFinish { .. }))?,
            StepState::Succeeded,
        ),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(terminal_result(&events)?, (SlotValue::I64(3), Taint::Clean));
    Ok(())
}

#[test]
fn owned_foreach_object_materialization_replays_handles() -> TestResult {
    let source = object_source();
    let join = control_step(&source, |kind| matches!(kind, K::ForEachJoin { .. }))?;
    let cut = cut_with(&source, (join, StepState::Succeeded), |store| {
        let key = store.insert_symbol("entry")?;
        let first = store.insert_object(Box::from([ObjectField::with_taint(
            key,
            SlotValue::I64(4),
            Taint::Secret,
        )]))?;
        let second = store.insert_object(Box::from([ObjectField::with_taint(
            key,
            SlotValue::I64(7),
            Taint::Clean,
        )]))?;
        let list = store.insert_list_with_taint(
            Box::from([SlotValue::Object(first), SlotValue::Object(second)]),
            Box::from([Taint::Secret, Taint::Clean]),
        )?;
        Ok((list, Taint::Secret))
    })?;
    let replay = replay_parity(&cut)?;
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    let output = cut
        .workflow
        .node(join)
        .and_then(|node| node.output.slot())
        .ok_or("missing ForEachJoin output")?;
    let SlotValue::List(list) = *replay.frame.read_slot(output)? else {
        return Err("ForEachJoin output is not a list".into());
    };
    let items = store.list(list)?;
    assert_eq!(items.len(), 2);
    let mut values = Vec::new();
    for item in items {
        let SlotValue::Object(object) = item else {
            return Err("ForEach object result is not an object".into());
        };
        let fields = store.object(*object)?;
        assert_eq!(fields.len(), 1);
        values.push(fields[0].value);
    }
    assert_eq!(values, [SlotValue::I64(4), SlotValue::I64(7)]);
    Ok(())
}

#[test]
fn owned_foreach_pending_ask_hydrates_without_reissue() -> TestResult {
    let source = ask_source();
    let ask = control_step(&source, |kind| matches!(kind, K::Ask { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Clean, Taint::Clean],
        (ask, StepState::Asking),
    )?;
    replay_parity(&cut)?;

    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let mut answers = [41_i64, 42_i64].into_iter();
    for _ in 0..256 {
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            let value = answers.next().ok_or("more ask tickets than answers")?;
            runtime.answer_ask(AskAnswer::new(
                ticket,
                ticket.answer_slot(),
                SlotValue::I64(value),
                Taint::Clean,
            ))?;
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert!(answers.next().is_none(), "answer list not consumed");
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::AskScheduledEvent { .. }))
            .count(),
        2,
        "recovery must not reissue an already recorded ask"
    );
    let replay = replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store))?
        .ok_or("owned ask replay not selected")?;
    let result = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("missing RunFinished")?;
    let SlotValue::List(list) = *replay.frame.read_slot(result)? else {
        return Err("ask result is not a list".into());
    };
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    assert_eq!(store.list(list)?, [SlotValue::I64(41), SlotValue::I64(42)]);
    Ok(())
}

#[test]
fn owned_foreach_action_recovers_recorded_completion_without_reissue() -> TestResult {
    let source = action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let cut = cut_at_actions(
        &source,
        &[SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Clean, Taint::Clean],
        (work, StepState::Running),
        &contracts,
    )?;
    let replay = replay_parity(&cut)?;
    assert!(
        replay
            .product
            .seed()
            .pending_actions
            .iter()
            .any(|entry| entry.step == work),
        "pending action must be retained before completion"
    );
    let output = cut
        .workflow
        .node(work)
        .and_then(|node| node.output.slot())
        .ok_or("missing Do output")?;

    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let mut values = [41_i64, 42_i64].into_iter();
    for _ in 0..256 {
        if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())) {
            let value = values.next().ok_or("more action tickets than outputs")?;
            runtime.complete_action_with_output(
                ticket,
                action_output(output, SlotValue::I64(value))?,
            )?;
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert!(values.next().is_none(), "action output list not consumed");
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        2,
        "recovery must not reissue a recorded action"
    );
    let replay = replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store))?
        .ok_or("owned action replay not selected")?;
    let result = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("missing RunFinished")?;
    let SlotValue::List(list) = *replay.frame.read_slot(result)? else {
        return Err("action result is not a list".into());
    };
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    assert_eq!(store.list(list)?, [SlotValue::I64(41), SlotValue::I64(42)]);
    Ok(())
}

#[test]
fn completed_same_step_actions_recover_through_public_runtime() -> TestResult {
    let source = action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let initial = cut_at_actions(
        &source,
        &[SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        &[Taint::Clean, Taint::Clean, Taint::Clean],
        (work, StepState::Running),
        &contracts,
    )?;
    let Cut {
        directory,
        workflow,
        roots,
        input_store,
        ..
    } = initial;
    let output = workflow
        .node(work)
        .and_then(|node| node.output.slot())
        .ok_or("missing Do output")?;
    let (live_frame, live_store, events) = {
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let mut runtime = runtime(journal.clone(), 1);
        assert!(matches!(
            runtime.recover_product(RUN)?,
            RuntimeRecoveryProduct::Resumable(_)
        ));
        runtime.recover_and_resume(RUN)?;
        let mut values = [41_i64, 42_i64].into_iter();
        let mut captured = None;
        for _ in 0..256 {
            if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())) {
                let value = values.next().ok_or("more pending actions than outputs")?;
                runtime.complete_action_with_output(
                    ticket,
                    action_output(output, SlotValue::I64(value))?,
                )?;
            }
            runtime.tick_all()?;
            assert_eq!(runtime.counters_snapshot().runs_failed, 0);
            let events = journal.events_for_run_full(RUN)?;
            if events
                .iter()
                .filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
                .count()
                >= 2
            {
                let state = runtime.inspect_run_state(RUN)?;
                captured = Some((
                    state.frame.checkpoint_v1(),
                    state.store.checkpoint_v1(),
                    events,
                ));
                break;
            }
        }
        captured.ok_or("second completed same-step action never reached")?
    };
    let cut = Cut {
        directory,
        workflow,
        roots,
        input_store,
        live_frame,
        live_store,
        events,
    };
    replay_parity(&cut)?;
    let mut bad_abi = cut.events.clone();
    if let Some(JournalEvent::ActionCompletedEnvelope {
        action_abi_digest, ..
    }) = bad_abi
        .iter_mut()
        .find(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
    {
        *action_abi_digest = WorkflowDigest::from_bytes([0; 32]);
    }
    assert!(matches!(
        replay_owned_for_each(&bad_abi, &cut.workflow, &cut.roots, Some(&cut.input_store)),
        Err(vb_storage::recovery::RecoveryError::ReplayDivergence { detail, .. })
            if detail.contains("action completion evidence mismatch")
    ));

    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let mut values = [43_i64].into_iter();
    for _ in 0..256 {
        if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())) {
            let value = values.next().ok_or("more pending actions than outputs")?;
            runtime.complete_action_with_output(
                ticket,
                action_output(output, SlotValue::I64(value))?,
            )?;
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert!(values.next().is_none(), "unexpected extra action request");
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        3,
        "recovery must not reissue completed activations"
    );
    Ok(())
}

#[test]
fn wait_resolution_prefix_recovers_and_resumes_after_reopen() -> TestResult {
    let source = wait_source();
    let wait = control_step(&source, |kind| matches!(kind, K::WaitUntil { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(0)],
        &[Taint::Clean],
        (wait, StepState::Succeeded),
    )?;
    assert!(
        cut.events
            .iter()
            .any(|event| matches!(event, JournalEvent::WaitResolvedEvent { .. })),
        "cut must include a recorded wait resolution"
    );
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    let (value, taint) = terminal_result(&events)?;
    assert!(matches!(value, SlotValue::List(_)));
    assert_eq!(taint, Taint::Clean);
    Ok(())
}

#[test]
fn timed_out_owned_ask_recovers_as_terminal_after_reopen() -> TestResult {
    let source = ask_timeout_source();
    let ask = control_step(&source, |kind| matches!(kind, K::Ask { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(0)],
        &[Taint::Clean],
        (ask, StepState::Asking),
    )?;
    {
        let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
        let mut runtime = runtime(journal.clone(), 1);
        assert!(matches!(
            runtime.recover_product(RUN)?,
            RuntimeRecoveryProduct::Resumable(_)
        ));
        runtime.recover_and_resume(RUN)?;
        for _ in 0..256 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_failed == 1 {
                break;
            }
        }
        assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    }
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let runtime = runtime(journal, 1);
    let product = runtime.recover_product(RUN)?;
    assert_eq!(product.cannot_resume_reason(), Some("terminal_state"));
    assert!(product.summary().terminal.is_some());
    Ok(())
}

#[test]
fn failed_owned_action_recovers_as_terminal_after_reopen() -> TestResult {
    let source = action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let cut = cut_at_actions(
        &source,
        &[SlotValue::I64(1)],
        &[Taint::Clean],
        (work, StepState::Running),
        &contracts,
    )?;
    {
        let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
        let mut runtime = runtime(journal, 1);
        assert!(matches!(
            runtime.recover_product(RUN)?,
            RuntimeRecoveryProduct::Resumable(_)
        ));
        runtime.recover_and_resume(RUN)?;
        runtime.tick_all()?;
        let ticket = runtime
            .lookup_pending_action_ticket(RUN, u64::from(work.get()))
            .ok_or("missing action ticket")?;
        runtime.fail_action(ticket, ActionFailure::from(ActionFailureCode::Rejected))?;
        for _ in 0..256 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_failed == 1 {
                break;
            }
        }
        assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    }
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let runtime = runtime(journal, 1);
    let product = runtime.recover_product(RUN)?;
    assert_eq!(product.cannot_resume_reason(), Some("terminal_state"));
    assert_eq!(product.summary().actions_resolved, 1);
    Ok(())
}

#[test]
fn owned_replay_without_step_start_uses_default_source_store() -> TestResult {
    let source = multi_source("count($step.loop)");
    let cut = cut_at(
        &source,
        &[SlotValue::I64(1)],
        &[Taint::Clean],
        (body_step(&source)?, StepState::Succeeded),
    )?;
    let prefix: Vec<JournalEvent> = cut
        .events
        .iter()
        .take_while(|event| !matches!(event, JournalEvent::StepStarted { .. }))
        .cloned()
        .collect();
    let replay = replay_owned_for_each(&prefix, &cut.workflow, &[], None)?
        .ok_or("default-source owned replay not selected")?;
    assert_eq!(replay.frame.pc(), cut.workflow.entry());
    assert_eq!(replay.product.summary().steps_started, 0);
    Ok(())
}

#[test]
fn accepted_for_each_limit_recovers_and_resumes_after_reopen() -> TestResult {
    let source = multi_source("count($step.loop)");
    let cut = cut_at(
        &source,
        &[
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
        ],
        &[Taint::Clean, Taint::Clean, Taint::Clean, Taint::Clean],
        (body_step(&source)?, StepState::Succeeded),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(terminal_result(&events)?, (SlotValue::I64(4), Taint::Clean));
    Ok(())
}

#[test]
fn owned_foreach_pending_wait_restores_timer_authority() -> TestResult {
    let source = wait_source();
    let wait = control_step(&source, |kind| matches!(kind, K::WaitUntil { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(60_000)],
        &[Taint::Clean],
        (wait, StepState::Waiting),
    )?;
    replay_parity(&cut)?;

    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    let product = runtime
        .recover_product(RUN)
        .map_err(|error| format!("wait recover_product: {error:?}"))?;
    assert!(matches!(product, RuntimeRecoveryProduct::Resumable(_)));
    runtime
        .recover_and_resume(RUN)
        .map_err(|error| format!("wait recover_and_resume: {error:?}"))?;
    runtime.tick_all()?;
    let timer = runtime
        .capture_timer_entry(RUN)
        .map_err(|error| format!("wait capture_timer_entry: {error:?}"))?;
    assert_eq!(timer.kind, PendingTimerKind::Wait);
    Ok(())
}

#[test]
fn corrupt_owned_foreach_evidence_is_rejected_typed() -> TestResult {
    let source = multi_source("count($step.loop)");
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let body = step_with(&workflow, |kind| matches!(kind, K::ForEachNext { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1)],
        &[Taint::Secret, Taint::Clean],
        (body, StepState::Succeeded),
    )?;
    let reject = |events: &[JournalEvent]| -> bool {
        matches!(
            replay_owned_for_each(events, &cut.workflow, &cut.roots, Some(&cut.input_store)),
            Err(vb_storage::recovery::RecoveryError::ReplayDivergence { .. })
                | Err(vb_storage::recovery::RecoveryError::CorruptSlotTaint { .. })
        )
    };

    let mut bad_value = cut.events.clone();
    if let Some(JournalEvent::SlotWrittenEvent {
        value: Some(bytes), ..
    }) = bad_value
        .iter_mut()
        .find(|event| matches!(event, JournalEvent::SlotWrittenEvent { value: Some(_), .. }))
    {
        bytes.clear();
    }
    assert!(reject(&bad_value), "corrupt value accepted");

    let mut missing_taint = cut.events.clone();
    if let Some(JournalEvent::SlotWrittenEvent { extra, .. }) = missing_taint
        .iter_mut()
        .find(|event| matches!(event, JournalEvent::SlotWrittenEvent { .. }))
    {
        *extra = None;
    }
    assert!(reject(&missing_taint), "missing taint accepted");

    let mut reordered = cut.events.clone();
    let starts: Vec<usize> = reordered
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event, JournalEvent::StepStarted { .. }))
        .map(|(index, _)| index)
        .collect();
    if let [first, second, ..] = starts.as_slice() {
        reordered.swap(*first, *second);
    }
    assert!(reject(&reordered), "reordered activation accepted");

    let mut corrupt_handle = cut.events.clone();
    let bad = postcard::to_allocvec(&SlotValue::List(ListId::new(9_999)))?;
    if let Some(JournalEvent::SlotWrittenEvent {
        value: Some(bytes), ..
    }) = corrupt_handle
        .iter_mut()
        .find(|event| matches!(event, JournalEvent::SlotWrittenEvent { value: Some(_), .. }))
    {
        *bytes = bad;
    }
    assert!(reject(&corrupt_handle), "corrupt handle accepted");

    let mut foreign_handle = cut.events.clone();
    let foreign = postcard::to_allocvec(&SlotValue::List(ListId::new(0)))?;
    if let Some(JournalEvent::SlotWrittenEvent {
        value: Some(bytes), ..
    }) = foreign_handle
        .iter_mut()
        .find(|event| matches!(event, JournalEvent::SlotWrittenEvent { value: Some(_), .. }))
    {
        *bytes = foreign;
    }
    assert!(reject(&foreign_handle), "foreign valid handle accepted");
    Ok(())
}

#[test]
fn owned_foreach_action_ticket_corruption_is_rejected_typed() -> TestResult {
    let source = action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let cut = cut_at_actions(
        &source,
        &[SlotValue::I64(1)],
        &[Taint::Clean],
        (work, StepState::Running),
        &contracts,
    )?;
    let mutate = |bad_run: bool, bad_attempt: bool, expected: &str| -> TestResult {
        let mut events = cut.events.clone();
        if let Some(JournalEvent::ActionScheduledTicket { ticket, .. }) = events
            .iter_mut()
            .find(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
        {
            if bad_run {
                ticket.run = RunId::new(9_999);
            }
            if bad_attempt {
                ticket.attempt = ticket.capacity.saturating_add(1);
            }
        }
        let result =
            replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store));
        assert!(
            matches!(
                result,
                Err(vb_storage::recovery::RecoveryError::ReplayDivergence { ref detail, .. })
                    if detail.contains(expected)
            ),
            "ticket corruption not rejected exactly: {result:?}"
        );
        Ok(())
    };
    mutate(true, false, "action ticket run mismatch")?;
    mutate(false, true, "action ticket attempt bounds invalid")?;
    Ok(())
}

#[test]
fn owned_foreach_limit_is_revalidated_during_replay() -> TestResult {
    let source = multi_source("count($step.loop)");
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let body = step_with(&workflow, |kind| matches!(kind, K::ForEachNext { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1)],
        &[Taint::Secret, Taint::Clean],
        (body, StepState::Succeeded),
    )?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = store.insert_list(Box::from([
        SlotValue::I64(3),
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(4),
        SlotValue::I64(5),
    ]))?;
    let input = workflow.input_slots().first().ok_or("input")?.slot;
    let roots = Box::from([(input, SlotValue::List(list), Taint::Clean)]);
    let checkpoint = store.checkpoint_v1();
    let result = replay_owned_for_each(&cut.events, &workflow, &roots, Some(&checkpoint));
    assert!(
        matches!(
            result,
            Err(vb_storage::recovery::RecoveryError::ReplayDivergence { .. })
        ),
        "over-limit source must be rejected during replay: {result:?}"
    );
    Ok(())
}
