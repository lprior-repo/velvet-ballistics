//! Staged model controls exercise real custody code through scripted external child I/O.
use super::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Clone, Copy)]
enum Step {
    Live,
    WaitError,
    Reaped(u8),
}
#[derive(Default)]
struct Trace {
    reaped: Cell<bool>,
    child_dropped: Cell<bool>,
    premature_drop: Cell<bool>,
    directory_lost: Cell<bool>,
    kill_without_live: Cell<bool>,
    kills: Cell<u32>,
    events: RefCell<Vec<&'static str>>,
}
struct ScriptedChild {
    steps: VecDeque<Step>,
    trace: Rc<Trace>,
    directory: std::path::PathBuf,
    fail_kill: bool,
    last_live: bool,
}
impl ChildControl for ScriptedChild {
    type Status = u8;
    fn try_wait(&mut self) -> std::io::Result<Option<u8>> {
        if !self.directory.exists() {
            self.trace.directory_lost.set(true);
        }
        match self.steps.pop_front() {
            Some(Step::Live) => {
                self.last_live = true;
                self.trace.events.borrow_mut().push("live");
                Ok(None)
            }
            Some(Step::WaitError) => {
                self.last_live = false;
                self.trace.events.borrow_mut().push("unknown");
                Err(std::io::Error::other("scripted wait failure"))
            }
            Some(Step::Reaped(status)) => {
                self.last_live = false;
                self.trace.reaped.set(true);
                self.trace.events.borrow_mut().push("reaped");
                Ok(Some(status))
            }
            None => {
                self.last_live = false;
                Err(std::io::Error::other("script exhausted without reap"))
            }
        }
    }
    fn kill(&mut self) -> std::io::Result<()> {
        if !self.last_live {
            self.trace.kill_without_live.set(true);
        }
        if !self.directory.exists() {
            self.trace.directory_lost.set(true);
        }
        self.trace
            .kills
            .set(self.trace.kills.get().saturating_add(1));
        self.trace.events.borrow_mut().push("kill");
        if self.fail_kill {
            Err(std::io::Error::other("scripted kill failure"))
        } else {
            Ok(())
        }
    }
}
impl Drop for ScriptedChild {
    fn drop(&mut self) {
        self.trace.child_dropped.set(true);
        if !self.trace.reaped.get() {
            self.trace.premature_drop.set(true);
        }
        if !self.directory.exists() {
            self.trace.directory_lost.set(true);
        }
    }
}
type ModelOwner = OwnedChild<ScriptedChild, tempfile::TempDir>;
fn fixture(
    root: &Path,
    steps: &[Step],
    fail_kill: bool,
) -> Result<(ModelOwner, Rc<Trace>, std::path::PathBuf), Box<dyn std::error::Error>> {
    let directory = tempfile::Builder::new()
        .prefix("watchdog-model-")
        .tempdir_in(root)?;
    let path = directory.path().to_path_buf();
    let trace = Rc::new(Trace::default());
    let child = ScriptedChild {
        steps: steps.iter().copied().collect(),
        trace: Rc::clone(&trace),
        directory: path.clone(),
        fail_kill,
        last_live: false,
    };
    Ok((
        OwnedChild {
            child,
            directory,
            reaped: false,
            kill_attempted: false,
            quarantined: false,
        },
        trace,
        path,
    ))
}
fn limits(initial: usize, reap: usize) -> WaitLimits {
    WaitLimits {
        initial,
        reap,
        pause: Duration::ZERO,
    }
}
fn check_released(trace: &Trace, path: &Path) {
    assert!(trace.reaped.get());
    assert!(trace.child_dropped.get());
    assert!(!trace.premature_drop.get());
    assert!(!trace.directory_lost.get());
    assert!(!trace.kill_without_live.get());
    assert!(!path.exists());
}
fn in_child(name: &str, body: fn(&Path) -> TestResult) -> TestResult {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return run_watchdog_child(name);
    };
    body(Path::new(&root))?;
    std::fs::write(Path::new(&root).join("child-completed"), name.as_bytes())?;
    Ok(())
}

fn normal_and_failed_cleanup(root: &Path) -> TestResult {
    let (mut owner, trace, path) = fixture(root, &[Step::Live, Step::Reaped(0)], false)?;
    let receipt = owner.reap(limits(2, 1));
    assert_eq!(receipt.status, 0);
    assert!(!receipt.notes.failed());
    assert!(!trace.child_dropped.get());
    assert!(path.exists());
    drop(owner);
    check_released(&trace, &path);
    let (mut owner, trace, path) = fixture(
        root,
        &[
            Step::WaitError,
            Step::Live,
            Step::Live,
            Step::Live,
            Step::Reaped(0),
        ],
        true,
    )?;
    let receipt = owner.reap(limits(0, 1));
    assert!(receipt.notes.failed());
    assert!(receipt.notes.wait_error.is_some());
    assert!(receipt.notes.kill_error.is_some());
    assert!(receipt.notes.quarantined);
    assert!(owner.quarantined);
    assert_eq!(trace.kills.get(), 1);
    assert_eq!(
        *trace.events.borrow(),
        vec!["unknown", "live", "kill", "live", "live", "reaped"]
    );
    assert!(!trace.child_dropped.get());
    assert!(path.exists());
    drop(owner);
    check_released(&trace, &path);
    Ok(())
}
fn exhausted_grace_is_not_release(root: &Path) -> TestResult {
    let (mut owner, trace, path) = fixture(
        root,
        &[Step::Live, Step::Live, Step::Live, Step::Reaped(0)],
        false,
    )?;
    let receipt = owner.reap(limits(0, 1));
    assert!(receipt.notes.intervention);
    assert!(receipt.notes.quarantined);
    assert_eq!(trace.kills.get(), 1);
    assert!(!trace.child_dropped.get());
    assert!(path.exists());
    drop(owner);
    check_released(&trace, &path);
    Ok(())
}
fn unwind_keeps_child_and_directory(root: &Path) -> TestResult {
    let (owner, trace, path) =
        fixture(root, &[Step::WaitError, Step::Live, Step::Reaped(0)], false)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let owner = owner;
        assert!(owner.reaped, "intentional post-spawn ownership unwind");
    }));
    assert!(result.is_err());
    assert_eq!(trace.kills.get(), 1);
    check_released(&trace, &path);
    Ok(())
}
struct EndlessReader {
    delivered: u64,
}
impl Read for EndlessReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        buffer.fill(0xff);
        self.delivered = self
            .delivered
            .saturating_add(u64::try_from(buffer.len()).map_err(std::io::Error::other)?);
        Ok(buffer.len())
    }
}
fn bounded_capture(root: &Path) -> TestResult {
    let mut reader = EndlessReader { delivered: 0 };
    let log = read_capped_reader(&mut reader, LOG_BYTES)?;
    assert_eq!(reader.delivered, LOG_BYTES);
    assert_eq!(u64::try_from(log.len())?, LOG_BYTES);
    let expanded = String::from_utf8_lossy(&log);
    assert!(
        u64::try_from(expanded.len())? <= LOG_BYTES.checked_mul(3).ok_or("text bound overflow")?
    );
    assert!(
        read_capped_reader(
            std::io::empty(),
            LOG_BYTES.checked_add(1).ok_or("limit overflow")?
        )
        .is_err()
    );
    let name = "known";
    let maximum = marker_limit(name)?;
    let bytes = read_capped_reader(std::io::Cursor::new(b"known-extra"), maximum)?;
    assert_ne!(bytes, name.as_bytes());
    assert_eq!(u64::try_from(bytes.len())?, maximum);
    let excessive = "x".repeat(
        MAX_MARKER_BYTES
            .checked_add(1)
            .ok_or("name size overflow")?,
    );
    assert!(marker_limit(&excessive).is_err());
    let (mut owner, trace, path) = fixture(root, &[Step::Reaped(0)], false)?;
    std::fs::write(
        path.join("child.log"),
        vec![0xff; usize::try_from(LOG_BYTES.checked_mul(2).ok_or("file size overflow")?)?],
    )?;
    let receipt = owner.reap(limits(0, 1));
    let log = read_capped(&receipt, &path.join("child.log"), LOG_BYTES)?;
    assert_eq!(u64::try_from(log.len())?, LOG_BYTES);
    assert!(read_capped(&receipt, &path, LOG_BYTES).is_err());
    assert!(read_capped(&receipt, &path.join("missing"), LOG_BYTES).is_err());
    drop(owner);
    check_released(&trace, &path);
    Ok(())
}

#[test]
fn watchdog_custody_survives_wait_and_kill_error_prefixes() -> TestResult {
    in_child(
        "journal::write_fence::test_support::custody_tests::watchdog_custody_survives_wait_and_kill_error_prefixes",
        normal_and_failed_cleanup,
    )
}
#[test]
fn watchdog_grace_exhaustion_retains_owner_until_reap() -> TestResult {
    in_child(
        "journal::write_fence::test_support::custody_tests::watchdog_grace_exhaustion_retains_owner_until_reap",
        exhausted_grace_is_not_release,
    )
}
#[test]
fn watchdog_postspawn_unwind_reaps_before_directory_drop() -> TestResult {
    in_child(
        "journal::write_fence::test_support::custody_tests::watchdog_postspawn_unwind_reaps_before_directory_drop",
        unwind_keeps_child_and_directory,
    )
}
#[test]
fn watchdog_capture_and_marker_reads_are_capped_after_reap() -> TestResult {
    in_child(
        "journal::write_fence::test_support::custody_tests::watchdog_capture_and_marker_reads_are_capped_after_reap",
        bounded_capture,
    )
}
