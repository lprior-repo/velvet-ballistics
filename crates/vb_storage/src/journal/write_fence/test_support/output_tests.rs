//! Staged output-budget controls; no OS-process result is inferred from these models.
use super::output::{CaptureSignals, STREAM_BYTES, capture, finish};
use super::{CHILD_ROOT, ChildControl, OwnedChild, TestResult, WaitLimits, run_watchdog_child};
use std::cell::Cell;
use std::io::{Cursor, Read};
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

// Shared watchdog contract: fixed retained and probe byte budgets.
const CONTRACT_RETAINED_BYTES: usize = 4_096;
const CONTRACT_PROBE_BYTES: usize = 4_097;

fn in_child(name: &str, body: fn(&Path) -> TestResult) -> TestResult {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return run_watchdog_child(name);
    };
    body(Path::new(&root))?;
    std::fs::write(Path::new(&root).join("child-completed"), name.as_bytes())?;
    Ok(())
}
fn exact_limits(_root: &Path) -> TestResult {
    let signals = CaptureSignals::default();
    let stdout = capture(Cursor::new(vec![b'o'; CONTRACT_RETAINED_BYTES]), &signals);
    let stderr = capture(Cursor::new(vec![b'e'; CONTRACT_RETAINED_BYTES]), &signals);
    assert_eq!(stdout.bytes().len(), CONTRACT_RETAINED_BYTES);
    assert_eq!(stderr.bytes().len(), CONTRACT_RETAINED_BYTES);
    assert!(!signals.failed());
    let log = finish(Ok(stdout), Ok(stderr))?;
    assert!(log.contains(&"o".repeat(CONTRACT_RETAINED_BYTES)));
    assert!(log.contains(&"e".repeat(CONTRACT_RETAINED_BYTES)));
    let signals = CaptureSignals::default();
    assert!(
        finish(
            Ok(capture(std::io::empty(), &signals)),
            Ok(capture(std::io::empty(), &signals))
        )
        .is_ok()
    );
    Ok(())
}
fn fixed_numeric_budget(_root: &Path) -> TestResult {
    assert_eq!(
        STREAM_BYTES, 4_096,
        "shared watchdog retained-byte contract changed"
    );
    let signals = CaptureSignals::default();
    let mut source = Cursor::new(vec![b'x'; CONTRACT_PROBE_BYTES]);
    let overflow = capture(&mut source, &signals);
    assert_eq!(source.position(), 4_097);
    assert_eq!(overflow.bytes().len(), 4_096);
    assert!(signals.failed());
    let empty = capture(std::io::empty(), &signals);
    assert!(finish(Ok(overflow), Ok(empty)).is_err());
    Ok(())
}
struct CountedReader {
    delivered: usize,
}
impl Read for CountedReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        buffer.fill(0xff);
        self.delivered = self
            .delivered
            .checked_add(buffer.len())
            .ok_or_else(|| std::io::Error::other("counter overflow"))?;
        Ok(buffer.len())
    }
}
fn overflow_is_failure(_root: &Path) -> TestResult {
    for stderr_overflow in [false, true] {
        let signals = CaptureSignals::default();
        let mut source = CountedReader { delivered: 0 };
        let overflow = capture(&mut source, &signals);
        assert_eq!(source.delivered, CONTRACT_PROBE_BYTES);
        assert_eq!(overflow.bytes().len(), CONTRACT_RETAINED_BYTES);
        assert!(signals.failed());
        let empty = capture(std::io::empty(), &signals);
        let result = if stderr_overflow {
            finish(Ok(empty), Ok(overflow))
        } else {
            finish(Ok(overflow), Ok(empty))
        };
        assert!(
            result.is_err(),
            "overflow must fail independently of child exit or marker success"
        );
    }
    Ok(())
}
struct BrokenReader;
impl Read for BrokenReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "staged read failure",
        ))
    }
}
fn read_and_worker_failure(_root: &Path) -> TestResult {
    let signals = CaptureSignals::default();
    let failed = capture(BrokenReader, &signals);
    assert!(signals.failed());
    assert!(finish(Ok(failed), Ok(capture(std::io::empty(), &signals))).is_err());
    let failed: std::thread::Result<_> = Err(Box::new("staged stdout worker panic"));
    assert!(finish(failed, Ok(capture(std::io::empty(), &signals))).is_err());
    let failed: std::thread::Result<_> = Err(Box::new("staged stderr worker panic"));
    assert!(finish(Ok(capture(std::io::empty(), &signals)), failed).is_err());
    Ok(())
}
struct StopChild {
    polls: usize,
    kills: Rc<Cell<usize>>,
}
impl ChildControl for StopChild {
    type Status = u8;
    fn try_wait(&mut self) -> std::io::Result<Option<u8>> {
        self.polls = self
            .polls
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("poll overflow"))?;
        Ok(if self.polls == 1 { None } else { Some(0) })
    }
    fn kill(&mut self) -> std::io::Result<()> {
        self.kills.set(self.kills.get().saturating_add(1));
        Ok(())
    }
}
fn overflow_requests_owned_cleanup(root: &Path) -> TestResult {
    let signals = CaptureSignals::default();
    let mut source = CountedReader { delivered: 0 };
    let _capture = capture(&mut source, &signals);
    let directory = tempfile::Builder::new()
        .prefix("watchdog-output-model-")
        .tempdir_in(root)?;
    let path = directory.path().to_owned();
    let kills = Rc::new(Cell::new(0));
    let child = StopChild {
        polls: 0,
        kills: Rc::clone(&kills),
    };
    let mut owned = OwnedChild {
        child,
        directory,
        reaped: false,
        kill_attempted: false,
        quarantined: false,
    };
    let confirmed = owned.reap_observing(
        WaitLimits {
            initial: 100,
            reap: 1,
            pause: Duration::ZERO,
        },
        || signals.failed(),
    );
    assert_eq!(confirmed.status, 0);
    assert!(confirmed.notes.intervention);
    assert!(confirmed.notes.failed());
    assert_eq!(kills.get(), 1);
    assert!(owned.reaped);
    assert!(path.exists());
    drop(owned);
    assert!(!path.exists());
    Ok(())
}
fn scoped_readers_report_both(_root: &Path) -> TestResult {
    let signals = CaptureSignals::default();
    let result = std::thread::scope(|scope| -> Result<String, String> {
        let observed = &signals;
        let stdout = std::thread::Builder::new()
            .spawn_scoped(scope, move || capture(Cursor::new(b"stdout"), observed))
            .map_err(|_| "stdout worker spawn failed")?;
        let stderr = std::thread::Builder::new()
            .spawn_scoped(scope, move || capture(Cursor::new(b"stderr"), observed))
            .map_err(|_| "stderr worker spawn failed")?;
        let stdout = stdout.join();
        let stderr = stderr.join();
        finish(stdout, stderr)
    })?;
    assert_eq!(result, "stdout:\nstdout\nstderr:\nstderr");
    assert!(!signals.failed());
    Ok(())
}

#[test]
fn watchdog_output_accepts_empty_and_exact_per_stream_budget() -> TestResult {
    in_child(
        "journal::write_fence::test_support::output_tests::watchdog_output_accepts_empty_and_exact_per_stream_budget",
        exact_limits,
    )
}
#[test]
fn watchdog_output_overflow_fails_on_either_stream_and_bounds_reads() -> TestResult {
    in_child(
        "journal::write_fence::test_support::output_tests::watchdog_output_overflow_fails_on_either_stream_and_bounds_reads",
        overflow_is_failure,
    )
}
#[test]
fn watchdog_output_read_and_worker_failures_cannot_succeed() -> TestResult {
    in_child(
        "journal::write_fence::test_support::output_tests::watchdog_output_read_and_worker_failures_cannot_succeed",
        read_and_worker_failure,
    )
}
#[test]
fn watchdog_output_overflow_requests_cleanup_before_normal_poll_budget() -> TestResult {
    in_child(
        "journal::write_fence::test_support::output_tests::watchdog_output_overflow_requests_cleanup_before_normal_poll_budget",
        overflow_requests_owned_cleanup,
    )
}
#[test]
fn watchdog_output_scoped_readers_collect_both_results() -> TestResult {
    in_child(
        "journal::write_fence::test_support::output_tests::watchdog_output_scoped_readers_collect_both_results",
        scoped_readers_report_both,
    )
}

#[test]
fn watchdog_output_budget_is_fixed_to_4096_retained_and_4097_probed() -> TestResult {
    in_child(
        "journal::write_fence::test_support::output_tests::watchdog_output_budget_is_fixed_to_4096_retained_and_4097_probed",
        fixed_numeric_budget,
    )
}
