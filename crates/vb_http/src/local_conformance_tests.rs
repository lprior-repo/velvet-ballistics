use super::*;
use crate::{HttpLimits, get_once};
use std::sync::mpsc;

const RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"total_count\":3}";

fn raw(endpoint: &PinnedGet, bytes: &[u8]) -> Result<Vec<u8>, FixtureError> {
    let mut stream = TcpStream::connect_timeout(&endpoint.address, IO_BUDGET)?;
    stream.set_read_timeout(Some(IO_BUDGET))?;
    stream.set_write_timeout(Some(IO_BUDGET))?;
    stream.write_all(bytes)?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut response = Vec::new();
    stream
        .take((RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut response)?;
    Ok(response)
}

#[test]
fn bounded_response_runs_real_transport_and_reaps_worker() -> Result<(), FixtureFailure> {
    let limits =
        HttpLimits::new(1024, 1024, Duration::from_millis(500)).map_err(FixtureError::Endpoint)?;
    let (response, capture) = with_response(RESPONSE, |endpoint| get_once(&endpoint, limits))?;
    let observed = response.map_err(|_| FixtureError::IncompleteRequest)?;
    assert_eq!(observed.status(), 200);
    assert_eq!(observed.body(), b"{\"total_count\":3}");
    assert!(capture.request.starts_with(b"GET /observed HTTP/1.1\r\n"));
    assert!(capture.request.ends_with(b"\r\n\r\n"));
    assert!(capture.request.len() <= REQUEST_BYTES);
    Ok(())
}

#[test]
fn oversized_response_rejects_before_operation_or_spawn() {
    static TOO_LARGE: [u8; RESPONSE_BYTES + 1] = [0; RESPONSE_BYTES + 1];
    let mut called = false;
    let result = with_response(&TOO_LARGE, |_| {
        called = true;
    });
    assert!(matches!(
        result,
        Err(FixtureFailure::Closed(FixtureError::Bounds))
    ));
    assert!(!called);
}

#[test]
fn pending_status_and_test_panic_preserve_static_custody_until_reap() -> Result<(), FixtureFailure>
{
    with_custody(|owner| {
        if owner.worker.is_some() {
            return Err(FixtureFailure::CleanupPending);
        }
        let (release, wait) = mpsc::channel::<()>();
        owner.worker = Some(
            thread::Builder::new()
                .spawn(move || {
                    wait.recv_timeout(Duration::from_secs(10))
                        .map_err(|_| FixtureError::Deadline)?;
                    Ok(Capture {
                        request: Vec::new(),
                        body_held: false,
                    })
                })
                .map_err(FixtureError::from)?,
        );
        // Hold suite serialization across this intentional failure/recovery pair.
        // The inner caller frame unwinds, while static custody retains the worker.
        let boundary = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert!(owner.try_reap().is_none());
            assert!(matches!(
                owner.wait_reap_polls(0),
                Err(FixtureFailure::CleanupPending)
            ));
            assert!(owner.worker.is_some());
            let result = with_owned_response(owner, RESPONSE, |_| ());
            assert!(matches!(result, Err(FixtureFailure::CleanupPending)));
            assert!(owner.worker.is_some());
            panic!("intentional caller-boundary failure after custody transfer");
        }));
        let signalled = release.send(());
        let capture = owner.wait_reap()?;
        assert!(owner.worker.is_none());
        assert!(capture.request.is_empty());
        assert!(boundary.is_err());
        assert!(signalled.is_ok());
        Ok(())
    })
}

#[test]
fn worker_panic_is_reaped_and_reported() -> Result<(), FixtureFailure> {
    with_custody(|owner| {
        if owner.worker.is_some() {
            return Err(FixtureFailure::CleanupPending);
        }
        owner.worker = Some(
            thread::Builder::new()
                .spawn(|| {
                    panic!("intentional owned-worker panic");
                })
                .map_err(FixtureError::from)?,
        );
        let result = owner.wait_reap();
        assert_eq!(
            result.as_ref().err(),
            Some(&FixtureFailure::Closed(FixtureError::WorkerPanic))
        );
        assert!(owner.worker.is_none());
        Ok(())
    })
}

#[test]
fn operation_panic_before_connect_preserves_worker_error_precedence() {
    let result = with_response(RESPONSE, |_| panic!("before connect"));
    assert!(matches!(
        result,
        Err(FixtureFailure::Closed(FixtureError::Deadline))
    ));
}

#[test]
fn operation_panic_after_response_is_reported_after_worker_reap() {
    let mut complete_response = false;
    let result = with_response(RESPONSE, |endpoint| {
        let response = raw(&endpoint, b"GET / HTTP/1.1\r\n\r\n");
        complete_response = response.as_ref().map(Vec::as_slice) == Ok(RESPONSE);
        panic!("after complete response");
    });
    assert!(complete_response);
    assert!(matches!(
        result,
        Err(FixtureFailure::Closed(FixtureError::OperationPanic))
    ));
}

#[test]
fn request_exact_limit_with_delimiter_and_response_exact_limit_succeed()
-> Result<(), FixtureFailure> {
    static RESPONSE_LIMIT: [u8; RESPONSE_BYTES] = [b'x'; RESPONSE_BYTES];
    let mut request = vec![b'a'; REQUEST_BYTES - 4];
    request.extend_from_slice(b"\r\n\r\n");
    let (response, capture) = with_response(&RESPONSE_LIMIT, |endpoint| raw(&endpoint, &request))?;
    assert_eq!(response?, RESPONSE_LIMIT);
    assert_eq!(capture.request, request);
    Ok(())
}

#[test]
fn request_delimiter_past_limit_and_full_limit_without_delimiter_reject() {
    for suffix in [b"\r\n\r\n".as_slice(), b"".as_slice()] {
        let mut request = vec![b'a'; REQUEST_BYTES];
        request.extend_from_slice(suffix);
        let result = with_response(RESPONSE, |endpoint| raw(&endpoint, &request));
        assert!(matches!(
            result,
            Err(FixtureFailure::Closed(FixtureError::Bounds))
        ));
    }
}

#[test]
fn incomplete_client_request_has_no_successful_capture() {
    let result = with_response(RESPONSE, |endpoint| raw(&endpoint, b"GET /"));
    assert!(matches!(
        result,
        Err(FixtureFailure::Closed(FixtureError::IncompleteRequest))
    ));
}

#[test]
fn queued_extra_connection_rejects_success() {
    let result = with_response(RESPONSE, |endpoint| -> Result<(), FixtureError> {
        let response = raw(&endpoint, b"GET / HTTP/1.1\r\n\r\n")?;
        assert_eq!(response, RESPONSE);
        let extra = TcpStream::connect_timeout(&endpoint.address, IO_BUDGET)?;
        // Completion of connect precedes releasing the fixture worker.
        drop(extra);
        Ok(())
    });
    assert!(matches!(
        result,
        Err(FixtureFailure::Closed(FixtureError::ExtraConnection))
    ));
}

#[test]
fn actual_mutex_poison_recovers_existing_worker_without_detach() -> Result<(), FixtureFailure> {
    with_serialized(|| {
        let (release, wait) = mpsc::channel::<()>();
        let boundary = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_custody_lock(&CUSTODY, |owner| -> Result<(), FixtureFailure> {
                if owner.worker.is_some() {
                    return Err(FixtureFailure::CleanupPending);
                }
                owner.worker = Some(
                    thread::Builder::new()
                        .spawn(move || {
                            wait.recv_timeout(Duration::from_secs(10))
                                .map_err(|_| FixtureError::Deadline)?;
                            Ok(Capture {
                                request: Vec::new(),
                                body_held: false,
                            })
                        })
                        .map_err(FixtureError::from)?,
                );
                panic!("intentional poison while static custody owns unfinished worker");
            })
        }));
        let poisoned = CUSTODY.is_poisoned();
        let signalled = release.send(());
        with_custody_lock(&CUSTODY, |owner| {
            let retained = owner.worker.is_some();
            let capture = owner.wait_reap()?;
            assert!(boundary.is_err());
            assert!(poisoned);
            assert!(retained);
            assert!(signalled.is_ok());
            assert!(capture.request.is_empty());
            assert!(owner.worker.is_none());
            Ok(())
        })
    })
}

#[test]
fn held_body_cleanup_pending_keeps_worker_until_explicit_reap() -> Result<(), FixtureFailure> {
    with_custody(|owner| {
        let limits = HttpLimits::new(1024, 1024, Duration::from_millis(500))
            .map_err(FixtureError::Endpoint)?;
        let mut saw_timeout_after_headers = false;
        let result = with_owned_behavior_reap_limit(
            owner,
            TIMED_HEADERS,
            BodyTiming::HoldUntilOperationReturns,
            0,
            |endpoint| {
                let (observed, parsed) =
                    crate::transport::local_observed_get_once(&endpoint, limits);
                saw_timeout_after_headers =
                    parsed && matches!(observed, Err(crate::AttemptFailure::Timeout));
            },
        );
        // Do not assert before reaping: a failed oracle must not skip cleanup.
        let pending_status = matches!(result, Err(FixtureFailure::CleanupPending));
        let retained = owner.worker.is_some();
        let capture = owner.wait_reap()?;
        assert!(saw_timeout_after_headers);
        assert!(pending_status);
        assert!(retained);
        assert!(capture.body_held);
        assert!(owner.worker.is_none());
        Ok(())
    })
}

fn completed_local_cleanup(
    operation: impl FnOnce() -> Result<Capture, FixtureError> + Send + 'static,
) -> Result<(LocalCleanupState, bool, bool, bool), FixtureError> {
    with_serialized(|| {
        let worker = thread::Builder::new()
            .spawn(operation)
            .map_err(FixtureError::from)?;
        let mut owner = CustodyOwner {
            worker: Some(worker),
        };
        let mut finished = false;
        for _ in 0..POLLS {
            if owner.worker.as_ref().is_some_and(JoinHandle::is_finished) {
                finished = true;
                break;
            }
            thread::sleep(POLL_DELAY);
        }
        let state = cleanup_state(&mut owner);
        let retained = owner.worker.is_some();
        // Even a failed completion oracle must not detach an owned test worker.
        let joined = owner
            .worker
            .take()
            .is_none_or(|worker| worker.join().is_ok());
        Ok((state, finished, retained, joined))
    })
}

#[test]
fn cleanup_state_reports_no_worker_for_isolated_empty_custody() {
    let mut owner = CustodyOwner { worker: None };
    assert_eq!(cleanup_state(&mut owner), LocalCleanupState::NoWorker);
    assert!(owner.worker.is_none());
}

#[test]
fn cleanup_state_pending_retains_real_local_worker_until_explicit_join() -> Result<(), FixtureError>
{
    with_serialized(|| {
        let (release, wait) = mpsc::channel::<()>();
        let worker = thread::Builder::new()
            .spawn(move || {
                wait.recv_timeout(Duration::from_secs(10))
                    .map_err(|_| FixtureError::Deadline)?;
                Ok(Capture {
                    request: Vec::new(),
                    body_held: false,
                })
            })
            .map_err(FixtureError::from)?;
        let mut owner = CustodyOwner {
            worker: Some(worker),
        };
        let state = cleanup_state(&mut owner);
        let retained = owner.worker.is_some();
        let signalled = release.send(());
        let reaped = owner.worker.take().map(JoinHandle::join);
        assert_eq!(state, LocalCleanupState::Pending);
        assert!(retained);
        assert!(signalled.is_ok());
        assert!(matches!(reaped, Some(Ok(Ok(_)))));
        assert_eq!(cleanup_state(&mut owner), LocalCleanupState::NoWorker);
        Ok(())
    })
}

#[test]
fn cleanup_state_reaps_successful_real_local_worker() -> Result<(), FixtureError> {
    let (state, finished, retained, joined) = completed_local_cleanup(|| {
        Ok(Capture {
            request: Vec::new(),
            body_held: false,
        })
    })?;
    assert!(finished);
    assert_eq!(state, LocalCleanupState::Reaped);
    assert!(!retained);
    assert!(joined);
    Ok(())
}

#[test]
fn cleanup_state_reaps_failed_real_local_worker() -> Result<(), FixtureError> {
    let (state, finished, retained, joined) =
        completed_local_cleanup(|| Err(FixtureError::Deadline))?;
    assert!(finished);
    assert_eq!(state, LocalCleanupState::ReapedWithFailure);
    assert!(!retained);
    assert!(joined);
    Ok(())
}

#[test]
fn cleanup_state_reaps_panicked_real_local_worker() -> Result<(), FixtureError> {
    let (state, finished, retained, joined) = completed_local_cleanup(|| {
        panic!("intentional isolated cleanup-state worker panic");
    })?;
    assert!(finished);
    assert_eq!(state, LocalCleanupState::ReapedWithFailure);
    assert!(!retained);
    assert!(joined);
    Ok(())
}

#[test]
fn public_cleanup_poll_observes_empty_state_without_recursive_serial_lock() {
    // Public polling acquires serialization itself; do not hold SERIAL here.
    assert_eq!(
        crate::poll_local_conformance_cleanup(),
        LocalCleanupState::NoWorker
    );
}
