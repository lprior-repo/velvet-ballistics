//! Owned child and directory custody require confirmed reap before capture or release.
use std::io::Read;
use std::path::Path;
use std::process::{Child, ExitStatus};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(crate) const CHILD_ROOT: &str = "VELVET_SHARED_WRITER_TEST_ROOT";
const POLL_STEP: Duration = Duration::from_millis(10);
const INITIAL_OBSERVATIONS: usize = 3_000;
const REAP_OBSERVATIONS: usize = 1_000;
const LOG_BYTES: u64 = 8_192;
const MAX_MARKER_BYTES: usize = 1_024;

trait ChildControl {
    type Status;
    fn try_wait(&mut self) -> std::io::Result<Option<Self::Status>>;
    fn kill(&mut self) -> std::io::Result<()>;
}
impl ChildControl for Child {
    type Status = ExitStatus;
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        Child::try_wait(self)
    }
    fn kill(&mut self) -> std::io::Result<()> {
        Child::kill(self)
    }
}

#[derive(Debug, Default)]
struct WaitNotes {
    intervention: bool,
    quarantined: bool,
    wait_error: Option<std::io::Error>,
    kill_error: Option<std::io::Error>,
}
impl WaitNotes {
    fn failed(&self) -> bool {
        self.intervention
            || self.quarantined
            || self.wait_error.is_some()
            || self.kill_error.is_some()
    }
}
struct ConfirmedReap<S> {
    status: S,
    notes: WaitNotes,
}
#[derive(Clone, Copy)]
struct WaitLimits {
    initial: usize,
    reap: usize,
    pause: Duration,
}
impl WaitLimits {
    fn normal() -> Self {
        Self {
            initial: INITIAL_OBSERVATIONS,
            reap: REAP_OBSERVATIONS,
            pause: POLL_STEP,
        }
    }
    fn cleanup() -> Self {
        Self {
            initial: 0,
            ..Self::normal()
        }
    }
}
struct OwnedChild<C: ChildControl, D> {
    child: C,
    directory: D,
    reaped: bool,
    kill_attempted: bool,
    quarantined: bool,
}
impl<C: ChildControl, D> OwnedChild<C, D> {
    fn reap(&mut self, limits: WaitLimits) -> ConfirmedReap<C::Status> {
        self.reap_observing(limits, || false)
    }
    fn reap_observing(
        &mut self,
        limits: WaitLimits,
        stop: impl Fn() -> bool,
    ) -> ConfirmedReap<C::Status> {
        let mut initial = limits.initial;
        let mut reap = limits.reap;
        let mut notes = WaitNotes::default();
        loop {
            if stop() {
                initial = 0;
            }
            let cleanup_active = self.kill_attempted;
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.reaped = true;
                    notes.quarantined = self.quarantined;
                    return ConfirmedReap { status, notes };
                }
                Ok(None) => {
                    if initial == 0 && !self.kill_attempted {
                        // A fresh live observation authorizes one request against this owned child.
                        notes.intervention = true;
                        self.kill_attempted = true;
                        if let Err(error) = self.child.kill() {
                            self.quarantined = true;
                            notes.kill_error = Some(error);
                        }
                    }
                }
                Err(error) => {
                    // Unknown status permits neither kill nor release of owned Store artifacts.
                    self.quarantined = true;
                    notes.wait_error = Some(error);
                }
            }
            if cleanup_active {
                reap = reap.saturating_sub(1);
                if reap == 0 {
                    self.quarantined = true;
                }
            }
            initial = initial.saturating_sub(1);
            // Permanent OS failure deliberately remains blocked with both fields owned.
            std::thread::sleep(limits.pause);
        }
    }
}
impl<C: ChildControl, D> Drop for OwnedChild<C, D> {
    fn drop(&mut self) {
        if !self.reaped {
            let _confirmed = self.reap(WaitLimits::cleanup());
        }
    }
}

fn marker_limit(name: &str) -> Result<u64, String> {
    if name.len() > MAX_MARKER_BYTES {
        return Err("owned child test name exceeds marker bound".into());
    }
    let length = name.len().checked_add(1).ok_or("marker bound overflow")?;
    u64::try_from(length).map_err(|_| "marker bound conversion failed".into())
}
fn read_capped_reader(reader: impl Read, limit: u64) -> std::io::Result<Vec<u8>> {
    if limit > LOG_BYTES {
        return Err(std::io::Error::other("capture limit exceeds fixed maximum"));
    }
    let mut bytes = Vec::new();
    reader.take(limit).read_to_end(&mut bytes)?;
    Ok(bytes)
}
fn read_capped<S>(
    _confirmed: &ConfirmedReap<S>,
    path: &Path,
    limit: u64,
) -> std::io::Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(std::io::Error::other(
            "owned child capture is not a regular file",
        ));
    }
    read_capped_reader(std::fs::File::open(path)?, limit)
}

pub(crate) fn run_watchdog_child(test_name: &str) -> TestResult {
    let marker_bytes = marker_limit(test_name)?;
    output::run(test_name, marker_bytes)
}

#[cfg(test)]
#[path = "test_support/custody_tests.rs"]
mod custody_tests;

#[path = "test_support/output.rs"]
mod output;
#[cfg(test)]
#[path = "test_support/output_tests.rs"]
mod output_tests;
