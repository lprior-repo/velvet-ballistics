use super::*;
use vb_core::frame::RunFrameCheckpointV1;

type Checkpoints = Vec<(RunFrameCheckpointV1, ValueStoreCheckpointV1)>;

fn runtime_at(journal: Arc<FjallJournal>, quantum: u64) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            max_active_runs: 1,
            step_budget_per_tick: quantum,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

fn trace(
    quantum: u64,
    reopen: bool,
    accepted_at_ms: u64,
) -> TestResult<(Checkpoints, Vec<JournalEvent>)> {
    let case = case(accepted_at_ms)?;
    let mut journal = open_journal(&case)?;
    let mut runtime = runtime_at(journal.clone(), quantum);
    runtime
        .activate_owned_submission(RUN, case.source_digest)
        .map_err(|e| format!("activate: {e:?}"))?;
    let mut checkpoints = Vec::new();
    let mut cut = false;
    for _ in 0..128 {
        runtime.tick_all().map_err(|e| format!("tick: {e:?}"))?;
        let events = journal
            .events_for_run_full(RUN)
            .map_err(|e| e.to_string())?;
        if runtime.counters_snapshot().runs_completed == 1 {
            assert!(cut, "must take a nonterminal lane cut");
            assert_lane_activity(
                &lane_activity(&events, case.body)?,
                case.body,
                case.output,
                &case.lane_items.map(|(value, _)| (value, Taint::Secret)),
            )?;
            return Ok((checkpoints, events));
        }
        let state = runtime
            .inspect_run_state(RUN)
            .map_err(|e| format!("inspect: {e:?}"))?;
        checkpoints.push((state.frame.checkpoint_v1(), state.store.checkpoint_v1()));
        let completed = events.iter().filter(|event| matches!(event, JournalEvent::StepSucceeded { step, .. } if *step == case.body)).count();
        if !cut && completed >= 1 {
            cut = true;
            if reopen {
                drop(runtime);
                drop(journal);
                journal = open_journal(&case)?;
                runtime = runtime_at(journal.clone(), quantum);
                let pending = runtime.pending_boundary_snapshot(64);
                let product = runtime
                    .recover_product(RUN)
                    .map_err(|e| format!("recover product: {e:?}"))?;
                assert!(product.is_resumable(), "{product:?}");
                assert_eq!(runtime.pending_boundary_snapshot(64), pending);
                assert_eq!(
                    journal
                        .events_for_run_full(RUN)
                        .map_err(|e| e.to_string())?,
                    events
                );
                runtime
                    .recover_and_resume(RUN)
                    .map_err(|e| format!("resume: {e:?}"))?;
            }
        }
    }
    Err(format!("budget {quantum} did not finish"))
}

#[test]
fn cold_reopen_matches_every_live_frame_store_and_event_at_budgets_one_two_four()
-> TestResult {
    let accepted_at_ms = now_ms()?;
    for quantum in [1, 2, 4] {
        let uninterrupted = trace(quantum, false, accepted_at_ms)?;
        let recovered = trace(quantum, true, accepted_at_ms)?;
        assert_eq!(
            recovered.0, uninterrupted.0,
            "all frame/store checkpoints budget={quantum}"
        );
        assert_eq!(
            recovered.1, uninterrupted.1,
            "all journal events budget={quantum}"
        );
    }
    Ok(())
}
