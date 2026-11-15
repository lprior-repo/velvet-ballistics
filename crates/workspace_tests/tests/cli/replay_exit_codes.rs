#[path = "replay_exit_codes/support.rs"]
mod support;
#[path = "replay_exit_codes/wire.rs"]
mod wire;

use std::process::{Command, Output};
use support::{Fixture, TestResult};
use vb_core::StepIdx;
use vb_storage::recovery::{ActionReplayTracker, RecoveryError, recover_full_journal};
use vb_storage::{EventSeq, FjallJournal, JournalError};

const CLEAN_TEXT: &str = "recovered 4 event(s) for run 8921\n  seq=0: RunAccepted\n  Unknown\n  Unknown\n  Unknown\nterminal: none\nBDD-KYYF-002 command=replay run_id=8921 evidence=.evidence/vb-kyyf/storage-replay-resume.md digest=normalized-replay events=4\n";

fn replay(fixture: &Fixture, emit: &str) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["replay", "8921", "--db"])
        .arg(&fixture.path)
        .args(["--emit", emit])
        .output()?)
}

fn assert_failure(fixture: &Fixture, code: i32, message: &str) -> TestResult {
    let before = fixture.records()?;
    for emit in ["text", "yaml", "postcard"] {
        let output = replay(fixture, emit)?;
        assert_eq!(output.status.code(), Some(code));
        assert!(output.stdout.is_empty());
        if emit == "text" {
            assert_eq!(String::from_utf8(output.stderr)?, format!("{message}\n"));
        } else {
            assert_eq!(
                wire::decode(&output.stderr, emit)?,
                serde_json::json!({
                    "success": false, "error": message
                })
            );
        }
        assert_eq!(fixture.records()?, before);
    }
    Ok(())
}

#[test]
fn current_issued_clean_replay_preserves_success_and_records() -> TestResult {
    let fixture = Fixture::new()?;
    let before = fixture.records()?;
    for emit in ["text", "yaml", "postcard"] {
        let output = replay(&fixture, emit)?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        if emit == "text" {
            assert_eq!(String::from_utf8(output.stdout)?, CLEAN_TEXT);
        } else {
            let report = wire::decode(&output.stdout, emit)?;
            support::assert_success_report(&fixture, &report)?;
        }
        assert_eq!(fixture.records()?, before);
    }
    Ok(())
}

#[test]
fn current_issued_replay_returns_eight_for_exact_typed_divergence() -> TestResult {
    let fixture = Fixture::new()?;
    let journal = FjallJournal::open(&fixture.path, None)?;
    journal.append_strict(&fixture.started(4, 1))?;
    journal.append_strict(&fixture.started(5, 0))?;
    assert!(matches!(recover_full_journal(&journal, fixture.run,
        &mut ActionReplayTracker::new(), &[], &[]),
        Err(RecoveryError::ReplayDivergence { step, detail })
        if step == StepIdx::ZERO && detail == "step 0 executed before previous step 1"));
    drop(journal);
    assert_failure(
        &fixture,
        8,
        "error replaying run 8921: replay divergence at step StepIdx(0): step 0 executed before previous step 1",
    )
}

#[test]
fn current_issued_sequence_gap_keeps_storage_exit_five() -> TestResult {
    let fixture = Fixture::new()?;
    let journal = FjallJournal::open(&fixture.path, None)?;
    journal.append_strict(&fixture.started(5, 0))?;
    let error = recover_full_journal(
        &journal,
        fixture.run,
        &mut ActionReplayTracker::new(),
        &[],
        &[],
    )
    .err()
    .ok_or("gap accepted")?;
    assert!(
        matches!(&error, RecoveryError::Journal(JournalError::SequenceGap { expected, actual })
        if *expected == EventSeq::new(4) && *actual == EventSeq::new(5))
    );
    let message = format!("error replaying run 8921: {error}");
    drop(journal);
    assert_failure(&fixture, 5, &message)
}

#[test]
fn real_storage_open_failure_keeps_exit_five_and_file_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("not-a-directory");
    std::fs::write(&path, b"retained storage failure sentinel")?;
    let error = FjallJournal::open(&path, None)
        .err()
        .ok_or("file opened as database")?;
    assert!(matches!(&error, JournalError::Fjall(_)));
    let fixture = Fixture::at_path(path);
    let message = format!(
        "error opening journal at {}: {error}",
        fixture.path.display()
    );
    for emit in ["text", "yaml", "postcard"] {
        let output = replay(&fixture, emit)?;
        wire::assert_open_error(output, emit, &message)?;
        assert_eq!(
            std::fs::read(&fixture.path)?,
            b"retained storage failure sentinel"
        );
    }
    Ok(())
}
