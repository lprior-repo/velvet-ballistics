use super::{Case, TestResult, goldens, support};
use vb_core::{CapabilitySet, ObjectId, SlotIdx, SlotValue, StepIdx, ValueStore};
use vb_runtime::counters::CounterSnapshot;
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeEngineError};
use vb_runtime::shard::types::RecoverRunCommand;
use vb_runtime::shard::{Shard, ShardPendingBoundarySnapshot};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, RunHeaderRecord, RunPolicyPin};

pub(super) struct RawBefore {
    journal: support::JournalSnapshot,
    counters: CounterSnapshot,
    boundary: ShardPendingBoundarySnapshot,
    retained: Vec<u8>,
    header: Option<RunHeaderRecord>,
}

impl RawBefore {
    pub(super) fn capture(shard: &Shard, case: &Case, journal: &FjallJournal) -> TestResult<Self> {
        let before = Self {
            journal: support::journal_snapshot(journal, case.run)?,
            counters: shard.counters().snapshot(),
            boundary: shard.pending_boundary_snapshot(0, 16),
            retained: case.retained_bytes(journal)?,
            header: journal.run_header(case.run)?,
        };
        assert_eq!(case.run, vb_core::RunId::new(17));
        assert_eq!(before.journal.0.len(), 3);
        assert_durable_pin(case, &before.journal.0)?;
        Ok(before)
    }

    pub(super) fn assert_original(
        &self,
        shard: &Shard,
        case: &Case,
        journal: &FjallJournal,
    ) -> TestResult {
        assert_eq!(support::journal_snapshot(journal, case.run)?, self.journal);
        assert_eq!(shard.counters().snapshot(), self.counters);
        assert_eq!(shard.pending_boundary_snapshot(0, 16), self.boundary);
        assert_eq!(shard.run_state_get(case.run), None);
        self.assert_retained(case, journal)
    }

    fn assert_retained(&self, case: &Case, journal: &FjallJournal) -> TestResult {
        assert_eq!(case.retained_bytes(journal)?, self.retained);
        assert_eq!(journal.run_header(case.run)?, self.header);
        assert_durable_pin(case, &journal.events_for_run_full(case.run)?)
    }
}

fn assert_durable_pin(case: &Case, events: &[JournalEvent]) -> TestResult {
    let Some(JournalEvent::RunAdmission {
        run,
        seq,
        policy_pin,
        artifact_digest,
        ..
    }) = events.get(1)
    else {
        return Err("original durable admission missing".into());
    };
    assert_eq!(*run, vb_core::RunId::new(17));
    assert_eq!(*seq, EventSeq::new(1));
    assert_eq!(*artifact_digest, case.artifact.digest);
    assert_eq!(
        *policy_pin,
        RunPolicyPin::Pinned(case.artifact.policy_digest)
    );
    Ok(())
}

pub(super) fn observe(
    shard: &mut Shard,
    case: &Case,
    journal: &FjallJournal,
    before: &RawBefore,
) -> TestResult {
    assert_eq!(before.boundary.command_queue_depth(), 0);
    assert_eq!(
        shard.pending_boundary_snapshot(0, 16).command_queue_depth(),
        1
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_failed(shard, case, journal, before)?;
    assert_eq!(shard.tick(), Ok(true));
    assert_failed(shard, case, journal, before)
}

fn assert_failed(
    shard: &Shard,
    case: &Case,
    journal: &FjallJournal,
    before: &RawBefore,
) -> TestResult {
    assert_eq!(
        support::journal_snapshot(journal, case.run)?,
        expected_journal(&before.journal)?
    );
    assert_eq!(
        shard.counters().snapshot(),
        failed_counters(before.counters)?
    );
    assert_eq!(shard.pending_boundary_snapshot(0, 16), before.boundary);
    assert_eq!(
        shard.pending_boundary_snapshot(0, 16).command_queue_depth(),
        0
    );
    assert_eq!(shard.pending_boundary_snapshot(0, 16).active_run_count(), 0);
    assert_eq!(shard.run_state_get(case.run), None);
    before.assert_retained(case, journal)
}

fn expected_journal(before: &support::JournalSnapshot) -> TestResult<support::JournalSnapshot> {
    let mut events = before.0.clone();
    events.extend([
        JournalEvent::StepStarted {
            run: vb_core::RunId::new(17),
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run: vb_core::RunId::new(17),
            seq: EventSeq::new(4),
            attempt: 1,
        },
    ]);
    let bytes = postcard::to_allocvec(&events)?;
    Ok((events, bytes))
}

fn failed_counters(mut before: CounterSnapshot) -> TestResult<CounterSnapshot> {
    before.runs_failed = before
        .runs_failed
        .checked_add(1)
        .ok_or("failed counter overflow")?;
    Ok(before)
}

pub(super) fn assert_driver_reason(case: &Case, command: &RecoverRunCommand) -> TestResult {
    goldens::assert_fresh_layout(&case.workflow)?;
    assert_eq!(command.frame.pc(), StepIdx::new(1));
    assert_eq!(
        command.frame.read_slot(SlotIdx::new(2)),
        Ok(&SlotValue::Object(ObjectId::new(0)))
    );
    assert_eq!(case.artifact.required_capabilities.len(), 0);
    assert_eq!(
        drive_raw(case, command),
        Err(RuntimeEngineError::Core(
            vb_core::engine::EngineError::ObjectOutOfBounds {
                object: ObjectId::new(0),
            }
        ))
    );
    Ok(())
}

fn drive_raw(
    case: &Case,
    command: &RecoverRunCommand,
) -> vb_runtime::engine::RuntimeEngineResult<vb_runtime::engine::RuntimeSignal> {
    let workflow = case.workflow.clone();
    let mut frame = command.frame.clone();
    let mut budget = vb_core::engine::StepBudget::new(1);
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let mut evidence = EvidenceCollector::new();
    let mut collect = command.collect_states.clone();
    let granted = CapabilitySet::from_grants(case.artifact.required_capabilities.clone());
    vb_runtime::engine::drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut budget,
        &mut store,
        &case.artifact.action_contracts,
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect,
        &granted,
        1,
    )
}
