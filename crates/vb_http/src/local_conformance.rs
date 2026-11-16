//! Owned loopback fixture for explicitly invoked, nondefault local conformance.
//! Not TLS, durable evidence, admission authority or a universal shutdown bound.

use crate::{PinnedGet, RequestRejection};
#[cfg(test)]
use std::io::{Read, Write};
#[cfg(test)]
use std::net::TcpStream;
use std::net::{Ipv4Addr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[path = "local_conformance_io.rs"]
mod fixture_io;
use fixture_io::serve;

const REQUEST_BYTES: usize = 8192;
const RESPONSE_BYTES: usize = 4096;
const POLLS: usize = 2000;
const IO_BUDGET: Duration = Duration::from_secs(1);
const POLL_DELAY: Duration = Duration::from_millis(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FixtureError {
    Io(std::io::ErrorKind),
    Endpoint(RequestRejection),
    Deadline,
    Bounds,
    Resources,
    IncompleteRequest,
    ExtraConnection,
    WorkerPanic,
    OperationPanic,
}

impl From<std::io::Error> for FixtureError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.kind())
    }
}

#[derive(Debug)]
pub(crate) struct Capture {
    pub request: Vec<u8>,
    pub body_held: bool,
}

/// Process-lifetime custody is deliberate: a test failure must not drop a live
/// JoinHandle into the harness. An occupied slot prevents another worker spawn.
/// Process exit is NOT a cleanup receipt. No universal join bound is claimed.
static SERIAL: Mutex<()> = Mutex::new(());
static CUSTODY: Mutex<CustodyOwner> = Mutex::new(CustodyOwner { worker: None });

#[derive(Debug)]
struct CustodyOwner {
    worker: Option<JoinHandle<Result<Capture, FixtureError>>>,
}

impl CustodyOwner {
    fn try_reap(&mut self) -> Option<Result<Capture, FixtureError>> {
        if !self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            return None;
        }
        self.worker.take().map(|worker| match worker.join() {
            Ok(result) => result,
            Err(_) => Err(FixtureError::WorkerPanic),
        })
    }

    fn wait_reap(&mut self) -> Result<Capture, FixtureFailure> {
        self.wait_reap_polls(POLLS)
    }

    fn wait_reap_polls(&mut self, polls: usize) -> Result<Capture, FixtureFailure> {
        for _ in 0..polls.min(POLLS) {
            if let Some(result) = self.try_reap() {
                return result.map_err(FixtureFailure::Closed);
            }
            thread::sleep(POLL_DELAY);
        }
        // The static owner still holds the handle; no failure value owns it.
        Err(FixtureFailure::CleanupPending)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "CleanupPending is not proof of resource release"]
pub(crate) enum FixtureFailure {
    Closed(FixtureError),
    CleanupPending,
}

/// Serialize test ownership; recover poison only to inspect preserved custody.
/// A pending worker blocks subsequent calls rather than being replaced/detached.
fn with_custody<T>(operation: impl FnOnce(&mut CustodyOwner) -> T) -> T {
    with_serialized(|| with_custody_lock(&CUSTODY, operation))
}

fn with_serialized<T>(operation: impl FnOnce() -> T) -> T {
    let _serial = match SERIAL.lock() {
        Ok(serial) => serial,
        Err(poison) => poison.into_inner(),
    };
    operation()
}

fn with_custody_lock<T>(
    custody: &'static Mutex<CustodyOwner>,
    operation: impl FnOnce(&mut CustodyOwner) -> T,
) -> T {
    let mut owner = match custody.lock() {
        Ok(owner) => owner,
        Err(poison) => poison.into_inner(),
    };
    operation(&mut owner)
}

impl From<FixtureError> for FixtureFailure {
    fn from(error: FixtureError) -> Self {
        Self::Closed(error)
    }
}

/// Private fixture; callers cannot select a remote address. Operation results are
/// returned only after worker reaping. The operation itself has no generic deadline
/// guarantee: eventual suite callers must use the bounded production HTTP operation.
#[cfg(test)]
pub(crate) fn with_response<T>(
    response: &'static [u8],
    operation: impl FnOnce(PinnedGet) -> T,
) -> Result<(T, Capture), FixtureFailure> {
    with_custody(|owner| with_owned_response(owner, response, operation))
}

fn with_owned_response<T>(
    owner: &mut CustodyOwner,
    response: &'static [u8],
    operation: impl FnOnce(PinnedGet) -> T,
) -> Result<(T, Capture), FixtureFailure> {
    with_owned_behavior(owner, response, BodyTiming::FixedResponse, operation)
}

#[derive(Clone, Copy)]
enum BodyTiming {
    FixedResponse,
    Immediate,
    HoldUntilOperationReturns,
}

const TIMED_HEADERS: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\nConnection: close\r\n\r\n";
const TIMED_BODY: &[u8] = b"{\"total_count\":3}";

fn with_owned_body_timing<T>(
    owner: &mut CustodyOwner,
    timing: BodyTiming,
    operation: impl FnOnce(PinnedGet) -> T,
) -> Result<(T, Capture), FixtureFailure> {
    with_owned_behavior(owner, TIMED_HEADERS, timing, operation)
}

fn with_owned_behavior<T>(
    owner: &mut CustodyOwner,
    response: &'static [u8],
    timing: BodyTiming,
    operation: impl FnOnce(PinnedGet) -> T,
) -> Result<(T, Capture), FixtureFailure> {
    with_owned_behavior_reap_limit(owner, response, timing, POLLS, operation)
}

fn with_owned_behavior_reap_limit<T>(
    owner: &mut CustodyOwner,
    response: &'static [u8],
    timing: BodyTiming,
    reap_polls: usize,
    operation: impl FnOnce(PinnedGet) -> T,
) -> Result<(T, Capture), FixtureFailure> {
    if owner.worker.is_some() {
        return Err(FixtureFailure::CleanupPending);
    }
    if response.len() > RESPONSE_BYTES {
        return Err(FixtureError::Bounds.into());
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(FixtureError::from)?;
    listener.set_nonblocking(true).map_err(FixtureError::from)?;
    let address = listener.local_addr().map_err(FixtureError::from)?;
    let endpoint = PinnedGet::fixture(address).map_err(FixtureError::Endpoint)?;
    let released = Arc::new(AtomicBool::new(false));
    let worker_release = Arc::clone(&released);
    let worker = thread::Builder::new()
        .spawn(move || serve(listener, response, worker_release, timing))
        .map_err(FixtureError::from)?;
    owner.worker = Some(worker);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(endpoint)));
    released.store(true, Ordering::Release);
    // Cleanup/worker failure has precedence over operation panic. In particular,
    // panic before connection yields worker Deadline, not OperationPanic.
    let capture = owner.wait_reap_polls(reap_polls)?;
    result
        .map(|value| (value, capture))
        .map_err(|_| FixtureError::OperationPanic.into())
}

#[cfg(test)]
#[path = "local_conformance_tests.rs"]
mod tests;

#[path = "local_conformance_cases.rs"]
mod cases;

pub use cases::{ExecutedLocalHttpConformance, LocalConformanceFailure, run_local_conformance};

/// Inspection only: no result grants authority or represents a completed suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCleanupState {
    NoWorker,
    Pending,
    Reaped,
    ReapedWithFailure,
}

/// Poll retained process-local work without starting network I/O.
/// Mutex acquisition and completed-worker join have no universal time bound.
pub fn poll_local_conformance_cleanup() -> LocalCleanupState {
    with_custody(cleanup_state)
}

fn cleanup_state(owner: &mut CustodyOwner) -> LocalCleanupState {
    if owner.worker.is_none() {
        return LocalCleanupState::NoWorker;
    }
    match owner.try_reap() {
        None => LocalCleanupState::Pending,
        Some(Ok(_)) => LocalCleanupState::Reaped,
        Some(Err(_)) => LocalCleanupState::ReapedWithFailure,
    }
}
