use super::{Capture, FixtureError, IO_BUDGET, POLL_DELAY, POLLS, REQUEST_BYTES, RESPONSE_BYTES};
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

fn accept(
    listener: &TcpListener,
    released: &AtomicBool,
    start: Instant,
) -> Result<Option<TcpStream>, FixtureError> {
    for _ in 0..POLLS {
        remaining(start)?;
        match listener.accept() {
            Ok((stream, _)) => return Ok(Some(stream)),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if released.load(Ordering::Acquire) {
                    return Ok(None);
                }
            }
            Err(error) => return Err(error.into()),
        }
        thread::sleep(POLL_DELAY);
    }
    Err(FixtureError::Deadline)
}

fn request(stream: &mut TcpStream, start: Instant) -> Result<String, FixtureError> {
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
            return String::from_utf8(bytes).map_err(|_| FixtureError::RequestEncoding);
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
        let written = match stream.write(pending) {
            Ok(0) => return Err(FixtureError::IncompleteRequest),
            Ok(written) => written,
            // Bounded production rejection may close before consuming supplied bytes.
            // Capture records the request and accepted count, not delivery of the body.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        pending = pending.get(written..).ok_or(FixtureError::Bounds)?;
    }
    Err(FixtureError::Bounds)
}

fn wait_release(released: &AtomicBool) -> Result<(), FixtureError> {
    for _ in 0..POLLS {
        if released.load(Ordering::Acquire) {
            return Ok(());
        }
        thread::sleep(POLL_DELAY);
    }
    Err(FixtureError::Deadline)
}

pub(super) fn serve(
    listener: TcpListener,
    response: Vec<u8>,
    withhold: bool,
    released: Arc<AtomicBool>,
) -> Result<Capture, FixtureError> {
    let start = Instant::now();
    let Some(mut stream) = accept(&listener, &released, start)? else {
        return Ok(Capture {
            accepted: 0,
            request: String::new(),
        });
    };
    stream.set_nonblocking(false)?;
    let request = request(&mut stream, start)?;
    if !withhold {
        respond(&mut stream, &response, start)?;
        drop(stream);
        wait_release(&released)?;
    } else {
        wait_release(&released)?;
        drop(stream);
    }
    match listener.accept() {
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(Capture {
            accepted: 1,
            request,
        }),
        Ok((extra, _)) => {
            drop(extra);
            Err(FixtureError::ExtraConnection)
        }
        Err(error) => Err(error.into()),
    }
}
