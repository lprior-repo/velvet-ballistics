#![forbid(unsafe_code)]

use vb_core::{ActionId, RunId, RuntimePolicy, StepIdx, WorkflowDigest};
use vb_storage::recovery::{ActionReplayTracker, RecoveryError, recover_full_journal};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, RunPolicyPin};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(41);
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: policy_pin\nwhen: {manual: {}}\nsteps:\n  - id: a\n    set: {output: a, value: '4'}\n  - id: b\n    set: {output: b, value: '8'}\n  - id: done\n    finish: {result: b}\n";

struct Fixture {
    _directory: tempfile::TempDir,
    journal: FjallJournal,
    pin: WorkflowDigest,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        let workflow = vb_compile::compile_workflow(SOURCE)?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[],
        )?;
        let pin = vb_storage::admission::compute_policy_digest(&workflow)?;
        assert_eq!(artifact.policy_digest, pin);
        let source = vb_storage::WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE).into()),
            source: SOURCE.to_vec(),
        };
        let inputs = vb_storage::RunInputsRecord::new(
            RUN,
            artifact.digest,
            source.digest,
            &workflow,
            Box::new([]),
        )?;
        let submission = vb_storage::RunSubmission::new(
            source,
            artifact,
            inputs,
            vb_storage::RunHeaderRecord {
                run: RUN,
                workflow_id: vb_core::WorkflowId::new(0),
                compiled_digest: workflow.digest(),
                status: 0,
                accepted_at_ms: u64::try_from(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_millis(),
                )?,
            },
        )?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        let events = journal.events_for_run_full(RUN)?;
        assert_eq!(events.len(), 4);
        assert!(
            matches!(events.get(1), Some(JournalEvent::RunAdmissionBudget { input_digest: vb_storage::RunInputBinding::Digest(_), policy_pin: RunPolicyPin::Pinned(actual), .. }) if *actual == pin)
        );
        assert!(
            matches!(events.last(), Some(JournalEvent::RunBudgetOriginEvent { started_millis, .. }) if *started_millis > 0)
        );
        Ok(Self {
            _directory: directory,
            journal,
            pin,
        })
    }

    fn raw(&self) -> TestResult<Vec<Vec<u8>>> {
        self.journal
            .events_for_run_full(RUN)?
            .iter()
            .map(|event| {
                self.journal
                    .get_event_bytes(RUN, event.seq())?
                    .ok_or_else(|| "missing raw event".into())
            })
            .collect()
    }
}

fn seeded_tracker() -> ActionReplayTracker {
    let mut tracker = ActionReplayTracker::new();
    tracker.mark_completed(ActionId::new(71), StepIdx::new(1));
    tracker.mark_failed(ActionId::new(72), StepIdx::new(2));
    tracker
}

fn assert_seed_preserved(tracker: &ActionReplayTracker) {
    assert!(tracker.has_completed(ActionId::new(71), StepIdx::new(1)));
    assert!(tracker.has_failed(ActionId::new(72), StepIdx::new(2)));
    assert!(!tracker.is_resolved(ActionId::new(73), StepIdx::ZERO));
}

#[test]
fn actual_pinned_policy_matches_nonempty_expectations_and_empty_behavior() -> TestResult {
    let fixture = Fixture::new()?;
    let expected = fixture.journal.events_for_run_full(RUN)?;
    let raw = fixture.raw()?;
    for policies in [
        vec![],
        vec![
            (StepIdx::new(2), fixture.pin),
            (StepIdx::new(1), fixture.pin),
        ],
    ] {
        let mut tracker = seeded_tracker();
        assert_eq!(
            recover_full_journal(&fixture.journal, RUN, &mut tracker, &[], &policies)?,
            expected
        );
        assert_seed_preserved(&tracker);
        assert_eq!(fixture.raw()?, raw);
    }
    let mut admission = expected.get(1).ok_or("admission")?.clone();
    let JournalEvent::RunAdmissionBudget { seq, .. } = &mut admission else {
        return Err("budget admission".into());
    };
    *seq = EventSeq::new(4);
    fixture.journal.append_strict(&admission)?;
    let repeated = fixture.journal.events_for_run_full(RUN)?;
    let raw = fixture.raw()?;
    let mut tracker = seeded_tracker();
    assert_eq!(
        recover_full_journal(
            &fixture.journal,
            RUN,
            &mut tracker,
            &[],
            &[(StepIdx::new(2), fixture.pin)]
        )?,
        repeated
    );
    assert_seed_preserved(&tracker);
    assert_eq!(fixture.raw()?, raw);
    Ok(())
}

#[test]
fn mismatched_policy_refuses_at_first_caller_label_before_tracker_or_journal_changes() -> TestResult
{
    let fixture = Fixture::new()?;
    let raw = fixture.raw()?;
    let profile_hash = WorkflowDigest::from_bytes(
        blake3::hash(&postcard::to_allocvec(&RuntimePolicy::Strict)?).into(),
    );
    assert_ne!(profile_hash, fixture.pin);
    for (policies, step) in [
        (vec![(StepIdx::new(2), profile_hash)], StepIdx::new(2)),
        (
            vec![
                (StepIdx::new(2), fixture.pin),
                (StepIdx::new(1), profile_hash),
                (StepIdx::ZERO, profile_hash),
            ],
            StepIdx::new(1),
        ),
    ] {
        let mut tracker = seeded_tracker();
        assert!(
            matches!(recover_full_journal(&fixture.journal, RUN, &mut tracker, &[], &policies), Err(RecoveryError::PolicyDigestMismatch { step: actual }) if actual == step)
        );
        assert_seed_preserved(&tracker);
        assert_eq!(fixture.raw()?, raw);
    }
    Ok(())
}

#[test]
fn unverified_or_conflicting_current_admission_cannot_satisfy_expectations() -> TestResult {
    for pin in [
        RunPolicyPin::Unverified,
        RunPolicyPin::Pinned(WorkflowDigest::from_bytes([0x99; 32])),
    ] {
        let fixture = Fixture::new()?;
        let mut admission = fixture
            .journal
            .events_for_run_full(RUN)?
            .get(1)
            .ok_or("admission")?
            .clone();
        let JournalEvent::RunAdmissionBudget {
            seq, policy_pin, ..
        } = &mut admission
        else {
            return Err("budget admission".into());
        };
        *seq = EventSeq::new(4);
        *policy_pin = pin;
        fixture.journal.append_strict(&admission)?;
        let raw = fixture.raw()?;
        let mut tracker = seeded_tracker();
        assert!(
            matches!(recover_full_journal(&fixture.journal, RUN, &mut tracker, &[], &[(StepIdx::new(2), fixture.pin)]), Err(RecoveryError::PolicyDigestMismatch { step }) if step == StepIdx::new(2))
        );
        assert_seed_preserved(&tracker);
        assert_eq!(fixture.raw()?, raw);
    }
    Ok(())
}

#[test]
fn missing_data_and_admission_keep_their_original_error_precedence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    let digest = WorkflowDigest::from_bytes([0x71; 32]);
    let mut tracker = seeded_tracker();
    let action = [(ActionId::new(3), digest)];
    let policy = [(StepIdx::new(2), digest)];
    assert!(
        matches!(recover_full_journal(&journal, RUN, &mut tracker, &action, &policy), Err(RecoveryError::NoRecoveryData { run }) if run == RUN)
    );
    assert_seed_preserved(&tracker);
    let accepted = JournalEvent::RunAccepted {
        run: RUN,
        seq: EventSeq::ZERO,
        workflow: digest,
    };
    journal.append_strict(&accepted)?;
    let before = journal.get_event_bytes(RUN, EventSeq::ZERO)?;
    assert!(
        matches!(recover_full_journal(&journal, RUN, &mut tracker, &action, &policy), Err(RecoveryError::PolicyDigestMismatch { step }) if step == StepIdx::ZERO)
    );
    assert_seed_preserved(&tracker);
    assert_eq!(journal.events_for_run_full(RUN)?, vec![accepted]);
    assert_eq!(journal.get_event_bytes(RUN, EventSeq::ZERO)?, before);
    Ok(())
}
