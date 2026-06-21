#[test]
fn runtime_timer_fired_returns_invalid_timer_fire_when_old_replaced_timer_event_arrives() -> Result<(), RuntimeError> {
    // Given a timed wait has captured a TimerFired authority token.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for stale replacement test");
    };
    let run = super::RunId::new(7_106);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let old_timer = match shard.pending_timer_get(run) {
        Some(timer) => timer,
        None => panic!("timed wait must register one pending timer before replacement"),
    };
    let stale_command = ShardCommand::TimerFired {
        run,
        generation: old_timer.generation,
        deadline: old_timer.deadline,
        kind: old_timer.kind,
    };
    let replacement_timer = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(99),
        kind: PendingTimerKind::Ask,
        generation: old_timer.generation.checked_add(1).unwrap_or(u64::MAX),
        deadline: std::time::Instant::now(),
    };
    assert_ne!(old_timer, replacement_timer);
    assert_eq!(
        shard.pending_timer_insert(run, replacement_timer),
        Some(old_timer)
    );
    let after_replacement_pending = shard.pending_timer_clone();
    assert_eq!(after_replacement_pending.len(), 1);
    assert_eq!(
        after_replacement_pending.get(&run).copied(),
        Some(replacement_timer)
    );

    // When the stale captured event arrives after the timer has been replaced.
    assert_eq!(shard.enqueue(stale_command), Ok(()));

    // Then stale delivery must be rejected exactly and the current timer must remain pending.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.pending_timers, after_replacement_pending);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    Ok(())
}

#[test]
fn runtime_run_only_timer_fired_fails_closed_without_consuming_live_timer() -> Result<(), RuntimeError> {
    // Given a public Runtime has a live wait timer with capturable authority.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return Ok(());
    };
    let mut runtime = crate::runtime::Runtime::new(shard_count, small_config())?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for runtime legacy test");
    };
    let run = super::RunId::new(7_206);
    assert_eq!(runtime.submit_direct(run, workflow), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    let captured = match runtime.capture_timer_entry(run) {
        Ok(entry) => entry,
        Err(error) => panic!("timer authority capture failed: {error}"),
    };

    // When legacy run-only delivery is called, it must fail closed rather than fabricate authority.
    assert_eq!(runtime.timer_fired(run), Err(RuntimeError::InvalidTimerFire));

    // Then the captured typed authority is still valid and can complete the run.
    assert_eq!(runtime.timer_entry_fired(captured), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn runtime_timer_fired_returns_invalid_timer_fire_when_cancelled_timer_event_arrives() -> Result<(), RuntimeError> {
    // Given a timed wait has a captured TimerFired authority token.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for stale cancel test");
    };
    let run = super::RunId::new(7_107);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let active_timer = match shard.pending_timer_get(run) {
        Some(timer) => timer,
        None => panic!("timed wait must register one pending timer before cancel"),
    };
    let stale_command = ShardCommand::TimerFired {
        run,
        generation: active_timer.generation,
        deadline: active_timer.deadline,
        kind: active_timer.kind,
    };
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 0);

    // When the stale captured event arrives after cancel.
    assert_eq!(shard.enqueue(stale_command), Ok(()));

    // Then stale delivery must map to InvalidTimerFire, not RunNotFound/no-op/success.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.pending_timer_get(run), None);
    assert_eq!(active_timer.kind, PendingTimerKind::Wait);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    Ok(())
}

#[test]
fn runtime_timer_fired_returns_invalid_timer_fire_when_terminal_timer_event_arrives() -> Result<(), RuntimeError> {
    // Given a timed wait run has already reached a terminal completed state.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for terminal stale test");
    };
    let run = super::RunId::new(7_108);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let terminal_timer = match shard.pending_timer_get(run) {
        Some(timer) => timer,
        None => panic!("timed wait must register one pending timer before valid fire"),
    };
    let terminal_command = ShardCommand::TimerFired {
        run,
        generation: terminal_timer.generation,
        deadline: terminal_timer.deadline,
        kind: terminal_timer.kind,
    };
    assert_eq!(shard.enqueue(terminal_command.clone()), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.pending_timer_get(run), None);
    assert_eq!(terminal_timer.kind, PendingTimerKind::Wait);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    // When a stale captured timer event targets the terminal run.
    assert_eq!(shard.enqueue(terminal_command), Ok(()));

    // Then it must be rejected as stale timer authority and must not resurrect/progress.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.pending_timer_get(run), None);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn timer_fired_command_exposes_generation_deadline_and_kind_authority_metadata() -> Result<(), RuntimeError> {
    // Given the public shard command that represents timer delivery.
    let run = super::RunId::new(7_109);
    let generation = 1_u64;
    let deadline = std::time::Instant::now();
    let kind = PendingTimerKind::Wait;
    let command = ShardCommand::TimerFired {
        run,
        generation,
        deadline,
        kind,
    };

    // Then State 10 must expose typed production-bound authority/freshness metadata.
    match command {
        ShardCommand::TimerFired {
            run: actual_run,
            generation: actual_generation,
            deadline: actual_deadline,
            kind: actual_kind,
        } => {
            assert_eq!(actual_run, run);
            assert_eq!(actual_generation, generation);
            assert_eq!(actual_deadline, deadline);
            assert_eq!(actual_kind, kind);
        }
        _ => panic!("TimerFired command must remain pattern-matchable"),
    }
    Ok(())
}

#[test]
fn timer_wheel_fired_entry_carries_freshness_metadata_for_runtime_validation() -> Result<(), RuntimeError> {
    // Given a wheel emits a due timer.
    let mut wheel = crate::shard::timer_wheel::TimerWheel::new();
    let deadline = std::time::Instant::now();
    let run = super::RunId::new(7_110);
    assert_eq!(wheel.insert(run, deadline, PendingTimerKind::Wait), Ok(()));

    // When the due entry is emitted.
    let fired = wheel.fire_expired(deadline);

    // Then runtime-visible timer delivery must include freshness authority metadata.
    assert_eq!(fired.len(), 1);
    let [entry] = fired.as_slice() else {
        panic!("exactly one timer entry must fire for authority metadata check");
    };
    assert_eq!(entry.run, run);
    assert_eq!(entry.generation, 1_u64);
    assert_eq!(entry.deadline, deadline);
    assert_eq!(entry.kind, PendingTimerKind::Wait);
    Ok(())
}

#[test]
fn shard_pending_timer_generation_overflow_fails_closed_without_wrap() -> Result<(), RuntimeError> {
    // Given a run is waiting and its live timer has reached the maximum generation.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for overflow test");
    };
    let run = super::RunId::new(7_207);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(mut timer) = shard.pending_timer_get(run) else {
        panic!("timed wait must register one pending timer before overflow test");
    };
    timer.generation = u64::MAX;
    assert_eq!(shard.pending_timer_insert(run, timer).is_some(), true);
    let Some(state) = shard.run_state_get(run).cloned() else {
        panic!("waiting run state must remain available for overflow test");
    };

    // When registration tries to replace the timer, generation must not wrap to 1.
    assert_eq!(
        shard.await_timer(run, state, PendingTimerKind::Ask, vb_core::ids::SlotIdx::ZERO),
        Err(RuntimeError::InvalidTimerFire)
    );

    // Then the existing max-generation timer and run state remain intact.
    assert_eq!(shard.pending_timer_get(run), Some(timer));
    assert_eq!(shard.run_state_contains(run), true);
    Ok(())
}

// RS-107 regression: an unrepresentable timer deadline (F64 value that
// saturates `deadline_ms` to `u64::MAX`) must NOT silently fall back to
// `Instant::now()` (the pre-fix bug), which would cause the timer to fire
// immediately and defeat the wait/ask contract.
//
// The fix surfaces `UnsupportedOperation { operation: "timer_deadline_overflow" }`
// when `Instant::checked_add` returns `None`. On platforms where Instant's
// representable range is wide enough to hold `u64::MAX` milliseconds (e.g.
// Linux's `CLOCK_MONOTONIC`), the checked_add succeeds and the timer is
// armed far in the future instead — both outcomes are correct fixes for
// the immediate-fire bug. This test asserts the immediate-fire outcome
// does not occur by verifying the deadline is at least ~584 billion
// years past `Instant::now()` (i.e. u64::MAX ms).
#[test]
fn shard_await_timer_does_not_fall_back_to_instant_now_on_overflow() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for deadline overflow test");
    };
    let run = super::RunId::new(7_213);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(initial_timer) = shard.pending_timer_get(run) else {
        panic!("timed wait must register one pending timer before deadline overflow test");
    };
    let Some(mut state) = shard.run_state_get(run).cloned() else {
        panic!("waiting run state must remain available for deadline overflow test");
    };

    // Overwrite the deadline slot with F64(f64::MAX), which `compute_deadline_ms_from_slot`
    // saturates to u64::MAX (~584 billion years in milliseconds).
    let huge = vb_core::value::FiniteF64::new(f64::MAX)
        .map_err(|error| RuntimeError::UnsupportedOperation {
            operation: "test_fixture_finite_f64_construction",
        })?;
    assert_eq!(
        state.frame.write_slot(vb_core::ids::SlotIdx::ZERO, vb_core::value::SlotValue::F64(huge)),
        Ok(())
    );

    let before = std::time::Instant::now();
    let result = shard.await_timer(run, state, PendingTimerKind::Wait, vb_core::ids::SlotIdx::ZERO);

    // Two valid outcomes:
    //   1. Platform Instant range can hold u64::MAX ms — await_timer
    //      succeeds and arms a timer far in the future.
    //   2. Platform Instant range cannot hold u64::MAX ms — await_timer
    //      returns the typed overflow error.
    match result {
        Ok(()) => {
            // Outcome 1: timer must be armed far in the future, NOT at
            // Instant::now() (the immediate-fire bug).
            let new_timer = shard.pending_timer_get(run).unwrap_or(initial_timer);
            assert_ne!(
                new_timer.deadline, before,
                "RS-107: huge deadline must not fall back to Instant::now()"
            );
            let minimum_future = before
                .checked_add(std::time::Duration::from_secs(86_400 * 365 * 100))
                .unwrap_or(before);
            assert!(
                new_timer.deadline > minimum_future,
                "RS-107: deadline must be at least a century past Instant::now(), got {:?} vs now {:?}",
                new_timer.deadline,
                before
            );
        }
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            // Outcome 2: the typed overflow error must carry the
            // expected operation code.
            assert_eq!(
                operation, "timer_deadline_overflow",
                "RS-107: overflow error must carry operation='timer_deadline_overflow', got '{operation}'"
            );
        }
        Err(other) => {
            panic!("RS-107: unexpected error variant: {other:?}");
        }
    }
    Ok(())
}

struct RejectTimerScheduledJournal {
    rejected_kind: PendingTimerKind,
    events: std::sync::Mutex<Vec<RuntimeJournalEvent>>,
}

impl RejectTimerScheduledJournal {
    fn shared(rejected_kind: PendingTimerKind) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            rejected_kind,
            events: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn snapshot(&self) -> crate::RuntimeResult<Vec<RuntimeJournalEvent>> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }
}

impl crate::journal::RuntimeJournal for RejectTimerScheduledJournal {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        let reject_wait = matches!(event, RuntimeJournalEvent::WaitScheduled { .. })
            && self.rejected_kind == PendingTimerKind::Wait;
        let reject_ask = matches!(event, RuntimeJournalEvent::AskScheduled { .. })
            && self.rejected_kind == PendingTimerKind::Ask;
        if reject_wait || reject_ask {
            return Err(RuntimeError::from(vb_storage::JournalError::QueueFull));
        }
        self.events
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

fn ask_timeout_only_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let set_timeout = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("ask_timeout_only"),
        digest: WorkflowDigest::from_bytes([9; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(0)),
            vb_core::value::ConstValue::I64(10),
        ]),
        slot_count: 2,
        symbols_count: 1,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn runtime_ask_timer_append_failure_does_not_register_pending_timer() -> Result<(), RuntimeError> {
    let journal = RejectTimerScheduledJournal::shared(PendingTimerKind::Ask);
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;
    let Some(workflow) = ask_timeout_only_workflow() else {
        panic!("ask_timeout_only_workflow fixture must compile for append failure test");
    };
    let run = super::RunId::new(7_208);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );

    let result = shard.tick();
    assert!(
        matches!(
            result,
            Err(RuntimeError::StorageJournalAppend { ref source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ),
        "expected StorageJournalAppend::QueueFull, got {result:?}, pending {:?}, events {:?}",
        shard.pending_timers,
        journal.snapshot()
    );

    assert_eq!(shard.pending_timer_get(run), None);
    assert_eq!(shard.run_state_contains(run), true);
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::RuntimeState::Initial)
    );
    match journal.snapshot() {
        Ok(events) => {
            assert_eq!(
                events
                    .iter()
                    .any(|event| matches!(event, RuntimeJournalEvent::AskScheduled { .. })),
                false
            );
        }
        Err(error) => panic!("journal snapshot failed: {error:?}"),
    }
    Ok(())
}

#[test]
fn runtime_timer_fired_rejects_wrong_generation_authority() -> Result<(), RuntimeError> {
    // Given a run with a live wait timer.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for generation mismatch test");
    };
    let run = super::RunId::new(7_111);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let pending_before = shard.pending_timer_clone();
    let Some(active_timer) = pending_before.get(&run).copied() else {
        panic!("timed wait must register one pending timer before generation mismatch test");
    };

    // When the delivery carries a generation that cannot match the captured timer.
    assert_eq!(
        shard.enqueue(ShardCommand::TimerFired {
            run,
            generation: u64::MAX,
            deadline: active_timer.deadline,
            kind: active_timer.kind,
        }),
        Ok(())
    );

    // Then it is stale authority and must not consume the live timer.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.pending_timers, pending_before);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    Ok(())
}

#[test]
fn runtime_timer_fired_rejects_wrong_deadline_authority() -> Result<(), RuntimeError> {
    // Given a run with a live wait timer.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for deadline mismatch test");
    };
    let run = super::RunId::new(7_112);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let pending_before = shard.pending_timer_clone();
    let Some(active_timer) = pending_before.get(&run).copied() else {
        panic!("timed wait must register one pending timer before deadline mismatch test");
    };
    let wrong_deadline = std::time::Instant::now() + std::time::Duration::from_secs(86_400);

    // When the delivery carries a deadline that cannot match the captured timer.
    assert_eq!(
        shard.enqueue(ShardCommand::TimerFired {
            run,
            generation: active_timer.generation,
            deadline: wrong_deadline,
            kind: active_timer.kind,
        }),
        Ok(())
    );

    // Then it is stale authority and must not consume the live timer.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.pending_timers, pending_before);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    Ok(())
}

#[test]
fn runtime_timer_fired_rejects_wrong_kind_authority() -> Result<(), RuntimeError> {
    // Given a run with a live wait timer.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        panic!("timed_wait_then_finish_workflow fixture must compile for kind mismatch test");
    };
    let run = super::RunId::new(7_113);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let pending_before = shard.pending_timer_clone();
    let Some(active_timer) = pending_before.get(&run).copied() else {
        panic!("timed wait must register one pending timer before kind mismatch test");
    };

    // When the delivery carries Ask authority for a Wait timer.
    assert_eq!(
        shard.enqueue(ShardCommand::TimerFired {
            run,
            generation: active_timer.generation,
            deadline: active_timer.deadline,
            kind: PendingTimerKind::Ask,
        }),
        Ok(())
    );

    // Then it is stale authority and must not consume the live timer.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.pending_timers, pending_before);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    Ok(())
}
