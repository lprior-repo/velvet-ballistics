#![forbid(unsafe_code)]
//! Runtime durable-recovery and test-util helpers.

#[cfg(feature = "test-util")]
use crate::admission;
#[cfg(feature = "test-util")]
use crate::journal::SharedRuntimeJournal;
#[cfg(feature = "test-util")]
use crate::primitives::collect::CollectStates;
#[cfg(feature = "test-util")]
use crate::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};
#[cfg(feature = "test-util")]
use crate::shard::Shard;
#[cfg(feature = "test-util")]
use crate::shard::timer::PendingTimerKind;
#[cfg(feature = "test-util")]
use crate::shard::{PendingTimer, RuntimeState};
#[cfg(feature = "test-util")]
use vb_core::StepIdx;
#[cfg(feature = "test-util")]
use vb_core::ids::RunId;

#[cfg(feature = "test-util")]
impl super::Runtime {
    /// Recovers all incomplete runs from the durable journal and rehydrates
    /// pending timers. Returns the list of rehydrated run IDs for observability.
    ///
    /// This is a hard, atomic operation: any error in hydration is propagated
    /// without partial state. On success, the runtime is ready to resume work
    /// across the new process boundary.
    ///
    /// Requires the `test-util` feature on `vb_core` (gated via `vb_runtime/test-util`).
    pub fn recover(&mut self, journal: &SharedRuntimeJournal) -> crate::RuntimeResult<Vec<RunId>> {
        let hydrations = vb_storage::recovery::recover_all_incomplete_runs(
            journal
                .storage_journal()
                .ok_or(crate::RuntimeError::InvalidRecoveryHydration)?
                .as_ref(),
        )
        .map_err(|_| crate::RuntimeError::InvalidRecoveryHydration)?;

        let mut recovered = Vec::with_capacity(hydrations.len());
        for hydration in hydrations {
            if let Some(run) = self.recover_one_run(journal, hydration)? {
                recovered.push(run);
            }
        }
        Ok(recovered)
    }

    /// Recovers a single run from a hydration seed.
    /// Returns the run ID if recovered, `None` if skipped.
    fn recover_one_run(
        &mut self,
        journal: &SharedRuntimeJournal,
        hydration: vb_storage::recovery::RecoveryHydration,
    ) -> crate::RuntimeResult<Option<RunId>> {
        let seed = match hydration {
            vb_storage::recovery::RecoveryHydration::FrameSeed(s) => s,
            _ => return Ok(None),
        };
        let run = seed.summary.run;
        let slot_count = seed.slot_count;
        let pc = seed.pc;
        // RQ-W0-16: durable summary distinguishes Cancelled, Killed, Finished,
        // and Failed, but rehydration silently reinserts the run as Resumable.
        // If a seed carries a terminal state (e.g. snapshot written before a
        // RunCancelled event was appended, or a direct recover_runtime_frame_seed
        // call), fail closed instead of pretending the run is live.
        if seed.summary.terminal.is_some() {
            return Err(crate::RuntimeError::InvalidRecoveryHydration);
        }
        // Persist a snapshot of the recovered seed so future recoveries can
        // short-circuit full event replay via `recover_snapshot_plus_tail`.
        // This is the only caller of the production snapshot write API.
        if let Some(fjall_journal) = journal.storage_journal()
            && seed.unsupported.is_fully_supported()
        {
            vb_storage::recovery::write_recovered_snapshot(fjall_journal.as_ref(), &seed)
                .map_err(|_| crate::RuntimeError::InvalidRecoveryHydration)?;
        }
        let frame = Self::hydrate_frame(seed)?;
        let pending_timer = Self::recover_timer_from_journal(journal, run, pc)?;
        Self::rehydrate_run_state(self, run, frame, slot_count, pending_timer)?;
        Ok(Some(run))
    }

    /// Hydrates a run frame from a recovery seed.
    fn hydrate_frame(
        seed: vb_storage::recovery::RecoveryFrameSeed,
    ) -> crate::RuntimeResult<vb_core::frame::RunFrame> {
        let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
        boundary.hydrate_run_frame()
    }

    /// Scans a run's journal for the last WaitScheduled or AskScheduled event.
    fn find_timer_event(
        events: &[vb_storage::JournalEvent],
        pc: StepIdx,
    ) -> Option<(&vb_storage::JournalEvent, StepIdx)> {
        events
            .iter()
            .rev()
            .find(|ev| Self::event_matches_step(ev, pc))
            .map(|ev| (ev, pc))
    }

    fn event_matches_step(ev: &vb_storage::JournalEvent, pc: StepIdx) -> bool {
        match ev {
            vb_storage::JournalEvent::WaitScheduledEvent { step: s, .. }
            | vb_storage::JournalEvent::AskScheduledEvent { step: s, .. } => pc == *s,
            _ => false,
        }
    }

    /// Extracts pending timer info from journal events for a suspended run.
    fn recover_timer_from_journal(
        journal: &SharedRuntimeJournal,
        run: RunId,
        pc: StepIdx,
    ) -> crate::RuntimeResult<Option<PendingTimer>> {
        let events = journal
            .storage_journal()
            .ok_or(crate::RuntimeError::InvalidRecoveryHydration)?
            .events_for_run(run)
            .map_err(|_| crate::RuntimeError::InvalidRecoveryHydration)?;
        Ok(Self::build_timer_from_event(Self::find_timer_event(
            &events, pc,
        )))
    }

    fn build_timer_from_event(
        event: Option<(&vb_storage::JournalEvent, StepIdx)>,
    ) -> Option<PendingTimer> {
        event.and_then(|(ev, pc)| match ev {
            vb_storage::JournalEvent::WaitScheduledEvent {
                step: s,
                deadline_ms,
                ..
            } if pc == *s => Some(Self::make_timer(*s, PendingTimerKind::Wait, *deadline_ms)),
            vb_storage::JournalEvent::AskScheduledEvent {
                step: s,
                deadline_ms,
                ..
            } if pc == *s => Some(Self::make_timer(*s, PendingTimerKind::Ask, *deadline_ms)),
            _ => None,
        })
    }

    fn make_timer(step: StepIdx, kind: PendingTimerKind, deadline_ms: u64) -> PendingTimer {
        PendingTimer {
            step,
            kind,
            generation: 0, // Updated by insert_timer
            deadline: std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(deadline_ms))
                .unwrap_or_else(std::time::Instant::now),
        }
    }

    /// Rehydrates a single run into its shard.
    fn rehydrate_run_state(
        &mut self,
        run: RunId,
        frame: vb_core::frame::RunFrame,
        slot_count: u16,
        pending_timer: Option<PendingTimer>,
    ) -> crate::RuntimeResult<()> {
        let shard_idx = self.shard_index(run);
        {
            let shard = self
                .shards
                .get_mut(shard_idx)
                .ok_or(crate::RuntimeError::RunNotFound)?;
            Self::insert_into_shard(shard, run, frame, slot_count);
        }
        if let Some(timer) = pending_timer {
            Self::insert_timer(self, run, shard_idx, timer)?;
        }
        Ok(())
    }

    fn insert_into_shard(
        shard: &mut Shard,
        run: RunId,
        frame: vb_core::frame::RunFrame,
        slot_count: u16,
    ) {
        shard.runtime_states.insert(run, RuntimeState::Resumable);
        let wf = admission::empty_workflow();
        shard.pending_workflows.insert(run, wf.clone());
        shard
            .runs
            .insert(run, Self::build_run_state(frame, wf, slot_count));
    }

    fn build_run_state(
        frame: vb_core::frame::RunFrame,
        workflow: vb_core::workflow::CompiledWorkflow,
        slot_count: u16,
    ) -> crate::shard::RunState {
        crate::shard::RunState {
            frame,
            workflow,
            store: vb_core::value_store::ValueStore::with_max_slots(slot_count),
            action_attempts: Box::new([]),
            admission: None,
            collect_states: CollectStates::default(),
            action_contracts: Box::new([]),
            last_snapshot_executed: 0,
        }
    }

    fn insert_timer(
        &mut self,
        run: RunId,
        shard_idx: usize,
        mut timer: PendingTimer,
    ) -> crate::RuntimeResult<()> {
        let shard = self
            .shards
            .get_mut(shard_idx)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        let generation = shard
            .next_pending_timer_generation(run)
            .ok_or(crate::RuntimeError::InvalidTimerFire)?;
        timer.generation = generation;
        shard.pending_timer_insert(run, timer);
        Ok(())
    }
}
