//! Production reactor regressions discovered by vb-cyz6d / vb-2a9i3.
use super::{IpcResponse, IpcServer};
use crate::{IpcCommand, IpcFrameHeader, MaxPayloadBytes};
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::os::unix::net::UnixStream;
use std::time::Duration;
use vb_runtime::{runtime::Runtime, shard::ShardConfig};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Runtime {
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    Runtime::new_for_tests_and_benchmarks_only(NonZeroUsize::MIN, config)
}

fn path(case: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "/tmp/vb_reactor_{case}_{}.sock",
        std::process::id()
    ))
}

fn health(correlation: u64, payload: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, correlation, u32::try_from(payload)?);
    let mut bytes = header.encode()?.to_vec();
    bytes.resize(
        bytes.len().checked_add(payload).ok_or("length overflow")?,
        0,
    );
    Ok(bytes)
}

fn poll(server: &mut IpcServer, runtime: &mut Runtime, turns: usize) -> TestResult {
    for _ in 0..turns {
        server.poll_once(runtime, Some(Duration::from_millis(1)))?;
    }
    Ok(())
}

fn response(
    client: &mut UnixStream,
) -> Result<(IpcFrameHeader, IpcResponse), Box<dyn std::error::Error>> {
    client.set_read_timeout(Some(Duration::from_millis(50)))?;
    let mut bytes = [0; crate::IPC_HEADER_LEN];
    client.read_exact(&mut bytes)?;
    let header = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT)?;
    let mut payload = vec![0; usize::try_from(header.payload_len)?];
    client.read_exact(&mut payload)?;
    Ok((header, postcard::from_bytes(&payload)?))
}

#[test]
fn complete_frame_larger_than_read_chunk_makes_progress() -> TestResult {
    let path = path("large");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut client = UnixStream::connect(&path)?;
    poll(&mut server, &mut runtime, 1)?;
    client.write_all(&health(7, 8192)?)?;
    poll(&mut server, &mut runtime, 20)?;
    let (header, payload) = response(&mut client)?;
    assert_eq!(header.correlation, 7);
    assert_eq!(payload, IpcResponse::Healthy);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn listener_drains_a_preconnected_burst() -> TestResult {
    let path = path("burst");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut clients = Vec::new();
    for correlation in 1..=3 {
        let mut client = UnixStream::connect(&path)?;
        client.write_all(&health(correlation, 0)?)?;
        clients.push((correlation, client));
    }
    poll(&mut server, &mut runtime, 20)?;
    for (correlation, mut client) in clients {
        let (header, payload) = response(&mut client)?;
        assert_eq!(
            (header.correlation, payload),
            (correlation, IpcResponse::Healthy)
        );
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn overload_rejects_extra_peer_and_existing_client_stays_usable() -> TestResult {
    let path = path("overload");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut clients = Vec::new();
    for _ in 0..256 {
        clients.push(UnixStream::connect(&path)?);
        poll(&mut server, &mut runtime, 1)?;
    }
    let mut extra = UnixStream::connect(&path)?;
    poll(&mut server, &mut runtime, 1)?;
    extra.set_read_timeout(Some(Duration::from_millis(50)))?;
    assert_eq!(extra.read(&mut [0; 1])?, 0);
    let first = clients.first_mut().ok_or("missing first client")?;
    first.write_all(&health(90, 0)?)?;
    poll(&mut server, &mut runtime, 2)?;
    assert_eq!(response(first)?.1, IpcResponse::Healthy);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn pending_response_cannot_be_replaced() -> TestResult {
    let (writer, _reader) = UnixStream::pair()?;
    writer.set_nonblocking(true)?;
    let mut writer = mio::net::UnixStream::from_std(writer);
    let poll = mio::Poll::new()?;
    poll.registry()
        .register(&mut writer, mio::Token(1), mio::Interest::READABLE)?;
    let filler = [0; 65_536];
    let mut blocked = false;
    for _ in 0..512 {
        match writer.write(&filler) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                blocked = true;
                break;
            }
            Err(e) => return Err(e.into()),
        }
    }
    assert!(blocked, "test must establish actual socket backpressure");
    let mut bytes = Vec::new();
    let first = IpcFrameHeader::new(IpcCommand::Health, 0, 11, 0);
    super::send_response(
        &mut writer,
        &mut bytes,
        poll.registry(),
        mio::Token(1),
        &first,
        &IpcResponse::Healthy,
    )?;
    let preserved = bytes.clone();
    let second = IpcFrameHeader::new(IpcCommand::Health, 0, 22, 0);
    let result = super::send_response(
        &mut writer,
        &mut bytes,
        poll.registry(),
        mio::Token(1),
        &second,
        &IpcResponse::Healthy,
    );
    assert_eq!(result, Err(super::IpcServerError::ResponsePending));
    assert_eq!(bytes, preserved);
    Ok(())
}

#[test]
fn shutdown_poll_waits_until_pending_acknowledgement_is_written() -> TestResult {
    let path = path("shutdown_pending");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut client = UnixStream::connect(&path)?;
    client.set_read_timeout(Some(Duration::from_millis(200)))?;
    poll(&mut server, &mut runtime, 1)?;
    let connection = server
        .clients
        .get_mut(&1)
        .ok_or("missing accepted client")?;
    let filler = [0; 65_536];
    let mut filled = 0usize;
    let mut blocked = false;
    for _ in 0..512 {
        match connection.stream.write(&filler) {
            Ok(count) => filled = filled.checked_add(count).ok_or("length overflow")?,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                blocked = true;
                break;
            }
            Err(error) => return Err(error.into()),
        }
    }
    assert!(blocked, "establish real outgoing socket backpressure");
    let shutdown = IpcFrameHeader::new(IpcCommand::Shutdown, 0, 88, 0);
    client.write_all(&shutdown.encode()?)?;
    assert!(server.poll_once(&mut runtime, Some(Duration::from_millis(1)))?);
    assert!(
        !server
            .clients
            .get(&1)
            .ok_or("connection retained")?
            .write_buffer
            .is_empty()
    );
    let mut discarded = vec![0; filled];
    client.read_exact(&mut discarded)?;
    let mut stopped = false;
    for _ in 0..20 {
        if !server.poll_once(&mut runtime, Some(Duration::from_millis(1)))? {
            stopped = true;
            break;
        }
    }
    let (header, payload) = response(&mut client)?;
    assert_eq!(
        (header.correlation, payload),
        (88, IpcResponse::ShuttingDown)
    );
    assert!(stopped, "completed acknowledgement ends shutdown polling");
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn invalid_magic_stops_after_four_buffered_bytes() -> TestResult {
    let path = path("magic_prefix");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut client = UnixStream::connect(&path)?;
    poll(&mut server, &mut runtime, 1)?;
    let mut frame = health(19, 8192)?;
    frame
        .get_mut(..4)
        .ok_or("missing magic")?
        .copy_from_slice(&0xDEAD_BEEF_u32.to_le_bytes());
    client.write_all(&frame)?;
    assert!(server.handle_readable(1, &mut runtime, None)?);
    let rejected = server
        .clients
        .get(&1)
        .ok_or("retained for inspection before removal")?;
    assert_eq!(
        rejected.read_buffer.as_slice(),
        &0xDEAD_BEEF_u32.to_le_bytes()
    );
    assert!(rejected.write_buffer.is_empty());
    server.remove_client(1);
    assert!(server.clients.is_empty());
    std::fs::remove_file(path)?;
    Ok(())
}
