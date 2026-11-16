//! Two scoped pipe readers bound live stdout/stderr without a child log file.
use super::{CHILD_ROOT, OwnedChild, TestResult, WaitLimits, read_capped};
use std::io::{ErrorKind, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) const STREAM_BYTES: usize = 4_096;
#[derive(Default)]
pub(super) struct CaptureSignals {
    failed: AtomicBool,
}
impl CaptureSignals {
    pub(super) fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}
pub(super) struct Capture {
    bytes: [u8; STREAM_BYTES],
    length: usize,
    overflow: bool,
    error: Option<ErrorKind>,
}
impl Capture {
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
}
pub(super) fn capture(mut reader: impl Read, signals: &CaptureSignals) -> Capture {
    // One extra byte detects overflow; valid reads make positive bounded progress.
    let mut probe = [0u8; STREAM_BYTES + 1];
    let mut used = 0usize;
    let error = loop {
        if used == probe.len() {
            break None;
        }
        match reader.read(&mut probe[used..]) {
            Ok(0) => break None,
            Ok(count) => match used.checked_add(count).filter(|end| *end <= probe.len()) {
                Some(end) => used = end,
                None => break Some(ErrorKind::InvalidData),
            },
            Err(error) => break Some(error.kind()),
        }
    };
    let overflow = used > STREAM_BYTES;
    let length = used.min(STREAM_BYTES);
    let mut bytes = [0u8; STREAM_BYTES];
    bytes[..length].copy_from_slice(&probe[..length]);
    if overflow || error.is_some() {
        signals.failed.store(true, Ordering::Release);
    }
    Capture {
        bytes,
        length,
        overflow,
        error,
    }
}
pub(super) fn finish(
    stdout: std::thread::Result<Capture>,
    stderr: std::thread::Result<Capture>,
) -> Result<String, String> {
    let stdout = stdout.map_err(|_| "stdout capture thread panicked".to_owned())?;
    let stderr = stderr.map_err(|_| "stderr capture thread panicked".to_owned())?;
    let log = format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(stdout.bytes()),
        String::from_utf8_lossy(stderr.bytes())
    );
    if stdout.overflow || stderr.overflow || stdout.error.is_some() || stderr.error.is_some() {
        return Err(format!(
            "owned child output capture failed: stdout overflow={}, error={:?}; stderr overflow={}, error={:?}\n{log}",
            stdout.overflow, stdout.error, stderr.overflow, stderr.error
        ));
    }
    Ok(log)
}

fn spawn_owned(
    test_name: &str,
) -> Result<OwnedChild<Child, tempfile::TempDir>, Box<dyn std::error::Error>> {
    let directory = tempfile::Builder::new()
        .prefix("velvet-shared-writer-child-")
        .tempdir_in(std::env::current_dir()?)?;
    let child = Command::new(std::env::current_exe()?)
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(CHILD_ROOT, directory.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // No fallible operation separates successful spawn from joint custody.
    Ok(OwnedChild {
        child,
        directory,
        reaped: false,
        kill_attempted: false,
        quarantined: false,
    })
}

pub(super) fn run(test_name: &str, marker_bytes: u64) -> TestResult {
    let signals = CaptureSignals::default();
    std::thread::scope(|scope| -> TestResult {
        let mut owned = spawn_owned(test_name)?;
        let stdout = owned
            .child
            .stdout
            .take()
            .ok_or("owned child stdout pipe missing")?;
        let stderr = owned
            .child
            .stderr
            .take()
            .ok_or("owned child stderr pipe missing")?;
        let observed = &signals;
        let stdout = std::thread::Builder::new()
            .name("watchdog-stdout".to_owned())
            .spawn_scoped(scope, move || capture(stdout, observed))?;
        let stderr = std::thread::Builder::new()
            .name("watchdog-stderr".to_owned())
            .spawn_scoped(scope, move || capture(stderr, observed))?;
        let confirmed = owned.reap_observing(WaitLimits::normal(), || signals.failed());
        // Collect both worker outcomes before fallible reporting. Scope also joins on error/unwind.
        let stdout = stdout.join();
        let stderr = stderr.join();
        let log = finish(stdout, stderr)?;
        if confirmed.notes.failed() {
            return Err(format!(
                "owned writer child required cleanup/quarantine: {:?}\n{log}",
                confirmed.notes
            )
            .into());
        }
        if !confirmed.status.success() {
            return Err(format!("owned writer child failed: {}\n{log}", confirmed.status).into());
        }
        let marker = read_capped(
            &confirmed,
            &owned.directory.path().join("child-completed"),
            marker_bytes,
        )?;
        if marker != test_name.as_bytes() {
            return Err(format!(
                "owned writer child completion marker mismatch ({} captured bytes)\n{log}",
                marker.len()
            )
            .into());
        }
        Ok(())
    })
}
