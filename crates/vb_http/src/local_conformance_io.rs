use super::{
    BodyTiming, Capture, FixtureError, IO_BUDGET, POLL_DELAY, POLLS, REQUEST_BYTES, RESPONSE_BYTES,
    TIMED_BODY,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn remaining(start: Instant) -> Result<Duration, FixtureError> {
    IO_BUDGET
        .checked_sub(start.elapsed())
        .filter(|duration| !duration.is_zero())
        .ok_or(FixtureError::Deadline)
}

fn accept(listener: &TcpListener, start: Instant) -> Result<TcpStream, FixtureError> {
    for _ in 0..POLLS {
        remaining(start)?;
        match listener.accept() {
            Ok((stream, _)) => return Ok(stream),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        thread::sleep(POLL_DELAY);
    }
    Err(FixtureError::Deadline)
}

fn request(stream: &mut TcpStream, start: Instant) -> Result<Vec<u8>, FixtureError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(REQUEST_BYTES)
        .map_err(|_| FixtureError::Resources)?;
    for _ in 0..REQUEST_BYTES {
        stream.set_read_timeout(Some(remaining(start)?))?;
        let mut byte = [0_u8; 1];
        if stream.read(&mut byte)? == 0 {
            return Err(FixtureError::IncompleteRequest);
        }
        bytes.extend_from_slice(&byte);
        if bytes.ends_with(b"\r\n\r\n") {
            return Ok(bytes);
        }
    }
    Err(FixtureError::Bounds)
}

fn respond(stream: &mut TcpStream, response: &[u8], start: Instant) -> Result<(), FixtureError> {
    let mut pending = response;
    for _ in 0..=RESPONSE_BYTES {
        if pending.is_empty() {
            return Ok(());
        }
        stream.set_write_timeout(Some(remaining(start)?))?;
        let written = stream.write(pending)?;
        if written == 0 {
            return Err(FixtureError::IncompleteRequest);
        }
        pending = pending.get(written..).ok_or(FixtureError::Bounds)?;
    }
    Err(FixtureError::Bounds)
}

pub(super) fn serve(
    listener: TcpListener,
    response: &'static [u8],
    released: Arc<AtomicBool>,
    timing: BodyTiming,
) -> Result<Capture, FixtureError> {
    let start = Instant::now();
    let mut stream = accept(&listener, start)?;
    stream.set_nonblocking(false)?;
    let request = request(&mut stream, start)?;
    respond(&mut stream, response, start)?;
    let body_held = matches!(timing, BodyTiming::HoldUntilOperationReturns);
    if matches!(timing, BodyTiming::Immediate) {
        respond(&mut stream, TIMED_BODY, start)?;
    }
    let mut stream = Some(stream);
    if !body_held {
        drop(stream.take());
    }
    // For held-body mode keep the socket open with exactly zero body bytes until
    // the bounded production operation finishes. No arbitrary caller delay exists.
    for _ in 0..POLLS {
        if released.load(Ordering::Acquire) {
            drop(stream.take());
            return match listener.accept() {
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    Ok(Capture { request, body_held })
                }
                Ok((extra, _)) => {
                    drop(extra);
                    Err(FixtureError::ExtraConnection)
                }
                Err(error) => Err(error.into()),
            };
        }
        thread::sleep(POLL_DELAY);
    }
    Err(FixtureError::Deadline)
}
