//! Test-only loopback fixture; capture does not prove response consumption.

use crate::{PinnedGet, RequestRejection};
use std::net::{Ipv4Addr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

mod io;

const REQUEST_BYTES: usize = 8192;
const RESPONSE_BYTES: usize = 1_114_112;
const IO_BUDGET: Duration = Duration::from_secs(2);
const POLL_DELAY: Duration = Duration::from_millis(1);
const POLLS: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FixtureError {
    #[error("fixture I/O failed: {0:?}")]
    Io(std::io::ErrorKind),
    #[error("fixture endpoint rejected: {0:?}")]
    Endpoint(RequestRejection),
    #[error("fixture budget expired")]
    Deadline,
    #[error("fixture byte limit exceeded")]
    Bounds,
    #[error("fixture allocation failed")]
    Resources,
    #[error("fixture request was incomplete")]
    IncompleteRequest,
    #[error("fixture request was not UTF-8")]
    RequestEncoding,
    #[error("fixture accepted an extra connection")]
    ExtraConnection,
    #[error("fixture worker unwound")]
    WorkerPanic,
    #[error("fixture operation unwound after worker cleanup")]
    OperationPanic,
    #[error("fixture cleanup remains in process-local custody")]
    CleanupPending,
}

impl From<std::io::Error> for FixtureError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.kind())
    }
}

#[derive(Debug)]
pub(crate) struct Capture {
    pub accepted: usize,
    pub request: String,
}

// Static custody survives caller failure; an unfinished handle is never detached.
static CUSTODY: Mutex<Owner> = Mutex::new(Owner { worker: None });

struct Owner {
    worker: Option<JoinHandle<Result<Capture, FixtureError>>>,
}

impl Owner {
    fn try_reap(&mut self) -> Option<Result<Capture, FixtureError>> {
        if !self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            return None;
        }
        self.worker.take().map(|worker| match worker.join() {
            Ok(result) => result,
            Err(_) => Err(FixtureError::WorkerPanic),
        })
    }

    fn ensure_clear(&mut self) -> Result<(), FixtureError> {
        if self.worker.is_none() {
            return Ok(());
        }
        match self.try_reap() {
            Some(result) => result.map(|_| ()),
            None => Err(FixtureError::CleanupPending),
        }
    }

    fn wait_reap(&mut self, polls: usize) -> Result<Capture, FixtureError> {
        for _ in 0..polls.min(POLLS) {
            if let Some(result) = self.try_reap() {
                return result;
            }
            thread::sleep(POLL_DELAY);
        }
        Err(FixtureError::CleanupPending)
    }
}

/// `false` sends supplied bytes then closes the response. `true`
/// withholds all response bytes until the operation returns, then closes.
/// The operation must itself be bounded and must not recursively call this helper.
/// Results are returned after reaping; mutex/scheduling/join have no universal bound.
pub(crate) fn with_response<T>(
    response: Vec<u8>,
    withhold: bool,
    operation: impl FnOnce(PinnedGet) -> T,
) -> Result<(T, Capture), FixtureError> {
    let mut owner = match CUSTODY.lock() {
        Ok(owner) => owner,
        Err(poison) => poison.into_inner(),
    };
    with_owned_response(&mut owner, response, withhold, operation)
}

fn with_owned_response<T>(
    owner: &mut Owner,
    response: Vec<u8>,
    withhold: bool,
    operation: impl FnOnce(PinnedGet) -> T,
) -> Result<(T, Capture), FixtureError> {
    owner.ensure_clear()?;
    if response.len() > RESPONSE_BYTES {
        return Err(FixtureError::Bounds);
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let endpoint = PinnedGet::fixture(listener.local_addr()?).map_err(FixtureError::Endpoint)?;
    let released = Arc::new(AtomicBool::new(false));
    let worker_release = Arc::clone(&released);
    owner.worker = Some(
        thread::Builder::new()
            .spawn(move || io::serve(listener, response, withhold, worker_release))?,
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(endpoint)));
    released.store(true, Ordering::Release);
    // Cleanup failure takes precedence; a pending handle remains in static custody.
    let capture = owner.wait_reap(POLLS)?;
    result
        .map(|value| (value, capture))
        .map_err(|_| FixtureError::OperationPanic)
}

#[cfg(test)]
mod tests;
