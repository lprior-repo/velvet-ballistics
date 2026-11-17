use super::*;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc;

const RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody";
const REQUEST: &[u8] = b"GET /observed HTTP/1.1\r\nHost: fixture.invalid\r\n\r\n";

fn raw(endpoint: &PinnedGet, request: &[u8]) -> Result<Vec<u8>, FixtureError> {
    let mut stream = TcpStream::connect_timeout(&endpoint.address, IO_BUDGET)?;
    stream.set_read_timeout(Some(IO_BUDGET))?;
    stream.set_write_timeout(Some(IO_BUDGET))?;
    stream.write_all(request)?;
    stream.shutdown(Shutdown::Write)?;
    let mut response = Vec::new();
    stream.take(1_114_113).read_to_end(&mut response)?;
    Ok(response)
}

#[test]
fn real_loopback_captures_request_and_reaps_before_return() -> Result<(), FixtureError> {
    let (response, capture) =
        with_response(RESPONSE.to_vec(), false, |endpoint| raw(&endpoint, REQUEST))?;
    assert_eq!(response?, RESPONSE);
    assert_eq!(capture.accepted, 1);
    assert_eq!(capture.request.as_bytes(), REQUEST);
    Ok(())
}

#[test]
fn operation_without_connection_returns_zero_actual_accepts() -> Result<(), FixtureError> {
    let (value, capture) = with_response(RESPONSE.to_vec(), false, |_| 42)?;
    assert_eq!(value, 42);
    assert_eq!(capture.accepted, 0);
    assert!(capture.request.is_empty());
    Ok(())
}

#[test]
fn oversized_response_rejects_without_invoking_operation() {
    let mut called = false;
    let result = with_response(vec![b'x'; 1_114_113], false, |_| called = true);
    assert!(matches!(result, Err(FixtureError::Bounds)));
    assert!(!called);
}

#[test]
fn exact_request_and_response_byte_caps_are_inclusive() -> Result<(), FixtureError> {
    let mut request = vec![b'a'; 8188];
    request.extend_from_slice(b"\r\n\r\n");
    let response = vec![b'x'; RESPONSE_BYTES];
    let (observed, capture) =
        with_response(response.clone(), false, |endpoint| raw(&endpoint, &request))?;
    assert_eq!(observed?, response);
    assert_eq!(capture.accepted, 1);
    assert_eq!(capture.request.as_bytes(), request.as_slice());
    Ok(())
}

#[test]
fn incomplete_or_overlimit_requests_cannot_produce_capture() {
    let mut over = vec![b'a'; REQUEST_BYTES];
    over.extend_from_slice(b"\r\n\r\n");
    let incomplete = with_response(RESPONSE.to_vec(), false, |endpoint| {
        raw(&endpoint, b"GET /")
    });
    assert!(matches!(incomplete, Err(FixtureError::IncompleteRequest)));
    let oversized = with_response(RESPONSE.to_vec(), false, |endpoint| raw(&endpoint, &over));
    assert!(matches!(oversized, Err(FixtureError::Bounds)));
}

#[test]
fn withhold_mode_sends_zero_response_bytes_before_operation_returns() -> Result<(), FixtureError> {
    let (observed, capture) = with_response(RESPONSE.to_vec(), true, |endpoint| {
        let mut stream = TcpStream::connect_timeout(&endpoint.address, IO_BUDGET)?;
        stream.set_write_timeout(Some(IO_BUDGET))?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;
        stream.write_all(REQUEST)?;
        let mut byte = [0_u8; 1];
        Ok::<_, FixtureError>(stream.read(&mut byte).err().map(|error| error.kind()))
    })?;
    assert!(matches!(
        observed?,
        Some(std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut),
    ));
    assert_eq!(capture.accepted, 1);
    assert_eq!(capture.request.as_bytes(), REQUEST);
    Ok(())
}

#[test]
fn queued_extra_connection_rejects_capture() {
    let result = with_response(RESPONSE.to_vec(), false, |endpoint| {
        let response = raw(&endpoint, REQUEST)?;
        let extra = TcpStream::connect_timeout(&endpoint.address, IO_BUDGET)?;
        drop(extra);
        Ok::<_, FixtureError>(response)
    });
    assert!(matches!(result, Err(FixtureError::ExtraConnection)));
}

#[test]
fn isolated_pending_owner_keeps_handle_and_blocks_replacement() -> Result<(), FixtureError> {
    let (release, wait) = mpsc::channel::<()>();
    static TEST_CUSTODY: Mutex<Owner> = Mutex::new(Owner { worker: None });
    let mut owner = match TEST_CUSTODY.lock() {
        Ok(owner) => owner,
        Err(poison) => poison.into_inner(),
    };
    owner.ensure_clear()?;
    owner.worker = Some(thread::Builder::new().spawn(move || {
        wait.recv_timeout(IO_BUDGET)
            .map_err(|_| FixtureError::Deadline)?;
        Ok(Capture {
            accepted: 0,
            request: String::new(),
        })
    })?);
    let pending = owner.wait_reap(0);
    let retained = owner.worker.is_some();
    let mut called = false;
    let blocked = with_owned_response(&mut owner, Vec::new(), false, |_| called = true);
    let signalled = release.send(()).is_ok();
    let reaped = owner.wait_reap(POLLS);
    // All assertions occur after the owned worker has had its cleanup opportunity.
    assert!(matches!(pending, Err(FixtureError::CleanupPending)));
    assert!(retained);
    assert!(matches!(blocked, Err(FixtureError::CleanupPending)));
    assert!(!called);
    assert!(signalled);
    assert_eq!(reaped?.accepted, 0);
    assert!(owner.worker.is_none());
    Ok(())
}

#[test]
fn operation_unwind_is_reported_after_actual_worker_reap() {
    let result = with_response::<()>(RESPONSE.to_vec(), false, |_| {
        std::panic::resume_unwind(Box::new("controlled operation unwind"));
    });
    assert!(matches!(result, Err(FixtureError::OperationPanic)));
}

#[test]
fn actual_worker_unwind_is_joined_and_reported() -> Result<(), FixtureError> {
    static TEST_CUSTODY: Mutex<Owner> = Mutex::new(Owner { worker: None });
    let mut owner = match TEST_CUSTODY.lock() {
        Ok(owner) => owner,
        Err(poison) => poison.into_inner(),
    };
    owner.ensure_clear()?;
    owner.worker = Some(thread::Builder::new().spawn(|| {
        std::panic::resume_unwind(Box::new("controlled worker unwind"));
    })?);
    let result = owner.wait_reap(POLLS);
    assert_eq!(result.err(), Some(FixtureError::WorkerPanic));
    assert!(owner.worker.is_none());
    Ok(())
}

#[test]
fn invalid_utf8_request_is_rejected_after_delimiter() {
    let result = with_response(RESPONSE.to_vec(), false, |endpoint| {
        raw(&endpoint, b"GET /observed HTTP/1.1\r\nX: \xff\r\n\r\n")
    });
    assert!(matches!(result, Err(FixtureError::RequestEncoding)));
}
