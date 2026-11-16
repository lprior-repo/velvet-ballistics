#![forbid(unsafe_code)]
//! Actual owned-process SIGKILL recovery and a separate clean-drop handle control.

#[path = "bin/wal_crash_helper.rs"]
mod wal_crash_helper;

use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use vb_core::RunId;
use vb_storage::{EventSeq, FjallJournal, JournalError, ReadOnlyJournal};
use wal_crash_helper::{HelperOp, HelperResponse, RESPONSE_MARKER, expected_events};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const CHILD_ENV: &str = "VB_WAL_CRASH_CHILD";
const CHILD_TEST: &str = "subprocess_wal_kill_then_reopen_recovers_all_events";

struct HelperChild {
    child: Child,
    channel: UnixStream,
}

impl HelperChild {
    fn start(op: &HelperOp) -> TestResult<Self> {
        let (channel, child_channel) = UnixStream::pair()?;
        channel.set_read_timeout(Some(Duration::from_secs(5)))?;
        channel.set_write_timeout(Some(Duration::from_secs(5)))?;
        let child = Command::new(std::env::current_exe()?)
            .args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"])
            .env(CHILD_ENV, "1")
            .stdin(Stdio::from(OwnedFd::from(child_channel.try_clone()?)))
            .stdout(Stdio::from(OwnedFd::from(child_channel)))
            .stderr(Stdio::piped())
            .spawn()?;
        let mut helper = Self { child, channel };
        let encoded = serde_json::to_vec(op)?;
        helper.channel.write_all(&encoded)?;
        helper.channel.write_all(b"\n")?;
        Ok(helper)
    }

    fn response(&mut self) -> TestResult<HelperResponse> {
        let mut reader = BufReader::new(&mut self.channel);
        for _ in 0..16 {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "child exited before WAL response",
                )
                .into());
            }
            if let Some((_, response)) = line.split_once(RESPONSE_MARKER) {
                return Ok(serde_json::from_str(response.trim_end())?);
            }
        }
        Err(Error::new(ErrorKind::InvalidData, "missing WAL response marker").into())
    }
}

impl Drop for HelperChild {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => eprintln!("WAL child status cleanup failed: {error}"),
        }
        if let Err(error) = self.child.kill() {
            eprintln!("WAL child kill cleanup failed: {error}");
        }
        if let Err(error) = self.child.wait() {
            eprintln!("WAL child wait cleanup failed: {error}");
        }
    }
}

fn assert_live_journal(path: &Path, child: &mut Child) -> TestResult {
    assert_eq!(
        child.try_wait()?,
        None,
        "commit child must still be running"
    );
    match FjallJournal::open(path, None) {
        Err(JournalError::ProcessLockHeld {
            holder_pid,
            path: lock_path,
            ..
        }) => {
            assert_eq!(holder_pid, Some(child.id()));
            assert_eq!(&*lock_path, path.join(".process.lock").as_path());
        }
        Err(JournalError::Fjall(fjall::Error::Locked)) => {}
        Err(error) => return Err(format!("unexpected live-lock result: {error}").into()),
        Ok(_) => return Err("commit child no longer holds the journal".into()),
    }
    let holder: u32 = std::fs::read_to_string(path.join(".process.lock"))?
        .trim()
        .parse()?;
    assert_eq!(holder, child.id());
    Ok(())
}

#[test]
fn in_process_strict_commit_survives_fresh_handle_replay() -> TestResult {
    let dir = tempfile::tempdir()?;
    let commit_count = 16u32;
    let expected = expected_events(commit_count);
    {
        let journal = FjallJournal::open(dir.path(), None)?;
        for event in &expected {
            journal.append_strict(event)?;
        }
        // This is a clean drop, intentionally separate from the SIGKILL scenario.
        drop(journal);
    }
    let journal = ReadOnlyJournal::open_inspect_view(dir.path())?;
    let replayed = journal.events_for_run(RunId::new(1))?;
    assert_eq!(u32::try_from(replayed.len())?, commit_count);
    assert_eq!(
        replayed.last().ok_or("nonempty replay")?.seq().get(),
        u64::from(commit_count.checked_sub(1).ok_or("nonzero count")?)
    );
    assert_eq!(replayed, expected);
    Ok(())
}

#[test]
fn subprocess_wal_kill_then_reopen_recovers_all_events() -> TestResult {
    if std::env::var_os(CHILD_ENV).is_some() {
        return wal_crash_helper::run();
    }
    let dir = tempfile::tempdir()?;
    let commit_count = 8u32;
    let last_seq = u64::from(commit_count.checked_sub(1).ok_or("nonzero count")?);
    let mut writer = HelperChild::start(&HelperOp::OpenAndCommit {
        path: dir.path().to_owned(),
        count: commit_count,
    })?;
    assert_eq!(
        writer.response()?,
        HelperResponse::Committed {
            count: commit_count,
            last_seq
        }
    );
    assert_live_journal(dir.path(), &mut writer.child)?;
    let killed_pid = writer.child.id();
    writer.child.kill()?;
    let killed = writer.child.wait()?;
    assert_eq!(
        killed.signal(),
        Some(9),
        "owned writer must terminate by SIGKILL"
    );
    eprintln!(
        "WAL owned child {killed_pid}: Strict commits={commit_count}, live journal lock verified, signal={:?}",
        killed.signal()
    );

    let mut reader = HelperChild::start(&HelperOp::ReopenAndRead {
        path: dir.path().to_owned(),
    })?;
    assert_ne!(reader.child.id(), killed_pid);
    match reader.response()? {
        HelperResponse::Replayed {
            count,
            last_seq: replayed_last,
            event_bytes,
        } => {
            assert_eq!(count, commit_count);
            assert_eq!(replayed_last, last_seq);
            let events: Vec<vb_storage::JournalEvent> = postcard::from_bytes(&event_bytes)?;
            assert_eq!(events, expected_events(commit_count));
        }
        response => return Err(format!("wrong replay response: {response:?}").into()),
    }
    let status = reader.child.wait()?;
    assert_eq!(
        status.code(),
        Some(0),
        "fresh replay child must complete successfully"
    );
    eprintln!(
        "WAL fresh child {}: exact records={commit_count}, last_seq={last_seq}, exit={:?}",
        reader.child.id(),
        status.code()
    );
    Ok(())
}

#[test]
fn subprocess_helper_setup_failure_is_not_a_successful_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let invalid_db = dir.path().join("regular-file");
    std::fs::write(&invalid_db, b"not a database directory")?;
    let mut helper = HelperChild::start(&HelperOp::OpenAndCommit {
        path: invalid_db,
        count: 8,
    })?;
    let response = helper.response();
    match response {
        Err(error) => assert_eq!(
            error.downcast_ref::<Error>().map(Error::kind),
            Some(ErrorKind::UnexpectedEof)
        ),
        Ok(response) => {
            return Err(format!("failed child acknowledged success: {response:?}").into());
        }
    }
    let status = helper.child.wait()?;
    assert_eq!(
        status.code(),
        Some(101),
        "helper error must fail its registered child test"
    );
    let mut stderr = String::new();
    helper
        .child
        .stderr
        .take()
        .ok_or("piped child stderr")?
        .read_to_string(&mut stderr)?;
    assert!(
        stderr.contains("open journal:"),
        "wrong child failure: {stderr}"
    );
    eprintln!(
        "WAL setup failure: child exit={:?}, no committed acknowledgement",
        status.code()
    );
    Ok(())
}

#[test]
fn event_seq_numbering_is_zero_indexed_and_dense() -> TestResult {
    let mut last = 0u64;
    let mut seq = EventSeq::new(0);
    assert_eq!(seq.get(), 0);
    for _ in 0..32u32 {
        assert_eq!(seq.get(), last, "seq must be dense and zero-indexed");
        last = last.checked_add(1).ok_or("sequence overflow")?;
        seq = EventSeq::new(last);
    }
    Ok(())
}
