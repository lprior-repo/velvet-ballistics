#[path = "recovery/lane_evidence.rs"]
mod lane_evidence;
#[path = "recovery_support.rs"]
mod support;
use super::{RunId, SlotValue, Taint};
use lane_evidence::{assert_lane_activity, lane_activity};
use std::num::NonZeroUsize;
use std::sync::Arc;
use support::{Case, case};
use vb_core::StepState;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent};

const RUN: RunId = RunId::new(41_702);
type TestResult<T = ()> = Result<T, String>;

struct Snapshot {
    cut_frame: vb_core::frame::RunFrameCheckpointV1,
    cut_store: ValueStoreCheckpointV1,
    final_frame: vb_core::frame::RunFrameCheckpointV1,
    final_store: ValueStoreCheckpointV1,
    values: Vec<(SlotValue, Taint)>,
    result_taint: Taint,
    lane_activity: Vec<JournalEvent>,
}

fn make_runtime(journal: Arc<FjallJournal>) -> TestResult<Runtime> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| "zero shard count".to_owned())?;
    Ok(Runtime::new(
        shard_count,
        ShardConfig {
            max_active_runs: 1,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    ))
}

fn open_journal(case: &Case) -> TestResult<Arc<FjallJournal>> {
    FjallJournal::open(case.directory.path(), None)
        .map(Arc::new)
        .map_err(|error| format!("open Fjall: {error}"))
}

fn execute(interrupt_at_cut: bool) -> TestResult<Snapshot> {
    let case = case()?;
    let journal = open_journal(&case)?;
    let mut runtime = make_runtime(journal.clone())?;
    runtime
        .activate_owned_submission(RUN, case.source_digest)
        .map_err(|error| format!("activate owned submission: {error:?}"))?;
    let mut cut = None;
    for _ in 0..256 {
        runtime
            .tick_all()
            .map_err(|error| format!("live owned tick: {error:?}"))?;
        let events = journal
            .events_for_run_full(RUN)
            .map_err(|error| format!("live journal events: {error}"))?;
        let completed = events
            .iter()
            .filter(|event| matches!(event, JournalEvent::StepSucceeded { step, .. } if *step == case.body))
            .count();
        if completed == 1 {
            let state = runtime
                .inspect_run_state(RUN)
                .map_err(|error| format!("inspect live cut: {error:?}"))?;
            cut = Some((state.frame.checkpoint_v1(), state.store.checkpoint_v1()));
            assert_eq!(
                state
                    .frame
                    .step_state(case.body)
                    .map_err(|error| format!("cut body state: {error}"))?,
                StepState::Succeeded
            );
            break;
        }
    }
    let (cut_frame, cut_store) =
        cut.ok_or_else(|| "runtime never reached owned nonterminal cut".to_owned())?;
    let live_cut_events = journal
        .events_for_run_full(RUN)
        .map_err(|error| format!("cut journal events: {error}"))?;
    assert!(
        !live_cut_events
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFinished { .. }))
    );
    if interrupt_at_cut {
        drop(runtime);
        drop(journal);
        let reopened = open_journal(&case)?;
        let mut recovered = make_runtime(reopened.clone())?;
        let product = recovered
            .recover_product(RUN)
            .map_err(|error| format!("recover_product: {error:?}"))?;
        assert!(
            matches!(&product, RuntimeRecoveryProduct::Resumable(_)),
            "owned concurrent submission must recover resumably: {product:?}"
        );
        recovered
            .recover_and_resume(RUN)
            .map_err(|error| format!("recover_and_resume: {error:?}"))?;
        recovered
            .tick_all()
            .map_err(|error| format!("activate recovered run: {error:?}"))?;
        finish(case, recovered, reopened, cut_frame, cut_store)
    } else {
        finish(case, runtime, journal, cut_frame, cut_store)
    }
}

fn finish(
    case: Case,
    mut runtime: Runtime,
    journal: Arc<FjallJournal>,
    cut_frame: vb_core::frame::RunFrameCheckpointV1,
    cut_store: ValueStoreCheckpointV1,
) -> TestResult<Snapshot> {
    let mut terminal = None;
    for _ in 0..256 {
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        let state = runtime
            .inspect_run_state(RUN)
            .map_err(|error| format!("inspect active run: {error:?}"))?;
        if state.frame.pc() == case.finish {
            let SlotValue::List(result) = *state
                .frame
                .read_slot(case.finish_result)
                .map_err(|error| format!("finish result slot: {error}"))?
            else {
                return Err("finish result is not a list".into());
            };
            let values = state
                .store
                .list_items_with_taint(result)
                .map_err(|error| format!("finish result list: {error}"))?
                .collect();
            let result_taint = state
                .frame
                .read_taint(case.finish_result)
                .map_err(|error| format!("finish result taint: {error}"))?;
            terminal = Some((
                state.frame.checkpoint_v1(),
                state.store.checkpoint_v1(),
                values,
                result_taint,
            ));
        }
        runtime
            .tick_all()
            .map_err(|error| format!("resume owned tick: {error:?}"))?;
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let (final_frame, final_store, values, result_taint) = terminal
        .ok_or_else(|| "finish state was not observed before terminal removal".to_owned())?;
    let events = journal
        .events_for_run_full(RUN)
        .map_err(|error| format!("terminal journal events: {error}"))?;
    let body_events = lane_activity(&events, case.body)?;
    assert_eq!(
        body_events
            .iter()
            .filter(|event| matches!(event, JournalEvent::StepStarted { .. }))
            .count(),
        3
    );
    assert_eq!(
        body_events
            .iter()
            .filter(|event| matches!(event, JournalEvent::StepSucceeded { .. }))
            .count(),
        3
    );
    assert_lane_activity(
        &body_events,
        case.body,
        case.output,
        &case.lane_items.map(|(value, _)| (value, Taint::Secret)),
    )?;
    Ok(Snapshot {
        cut_frame,
        cut_store,
        final_frame,
        final_store,
        values,
        result_taint,
        lane_activity: body_events,
    })
}

#[test]
fn owned_at_once_two_strict_fjall_cold_recovery_matches_uninterrupted_run() -> TestResult {
    let uninterrupted = execute(false)?;
    let recovered = execute(true)?;
    assert_eq!(recovered.cut_frame, uninterrupted.cut_frame);
    assert_eq!(recovered.cut_store, uninterrupted.cut_store);
    assert_eq!(recovered.final_frame, uninterrupted.final_frame);
    assert_eq!(recovered.final_store, uninterrupted.final_store);
    assert_eq!(recovered.values, uninterrupted.values);
    assert_eq!(
        recovered.values,
        vec![
            (SlotValue::I64(11), Taint::Secret),
            (SlotValue::I64(22), Taint::Secret),
            (SlotValue::I64(33), Taint::Secret),
        ]
    );
    assert_eq!(recovered.result_taint, uninterrupted.result_taint);
    assert_eq!(recovered.result_taint, Taint::Secret);
    assert_eq!(recovered.lane_activity, uninterrupted.lane_activity);
    Ok(())
}

#[path = "recovery/budgets.rs"]
mod budgets;

#[path = "recovery/corruption.rs"]
mod corruption;
