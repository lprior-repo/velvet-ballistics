use super::{CleanupPath, IpcServer, UnixStream, temp_socket_path};
use std::io::{ErrorKind, Read};
use std::time::{Duration, Instant};

const PAYLOAD_BYTES: usize = 10_485_760;
const BLOCK_COUNT: usize = 2_621_440;
const READ_BUFFER_BYTES: usize = 16_384;
const MAX_READS_PER_TURN: usize = 1_024;
const MAX_DRAIN_TURNS: usize = 4_096;
const DEADLINE: Duration = Duration::from_secs(2);

#[derive(Debug, Eq, PartialEq)]
enum OracleFailure {
    Allocation,
    Arithmetic,
    SliceBounds,
    TempDirMissing,
    SocketPathTooLong,
    Bind,
    Connect,
    Accept,
    MissingClient,
    DirtyQueue,
    Handler,
    ConnectionClosed,
    NoPositiveProgress,
    FullWrite,
    PeerEof,
    PeerRead,
    PrefixMismatch,
    SuffixMismatch,
    ReadLimit,
    Deadline,
    DrainLimit,
    FinalStreamMismatch,
    ExtraData,
    FinalRead,
}

#[test]
fn handle_writable_preserves_exact_bytes_after_positive_partial_progress() {
    assert_eq!(run_oracle(), Ok(()), "partial-write oracle failure");
}
fn run_oracle() -> Result<(), OracleFailure> {
    let original = encoded_blocks()?;
    let configured_temp = std::env::var_os("TMPDIR").ok_or(OracleFailure::TempDirMissing)?;
    let path = temp_socket_path("partial_write_exact");
    if !path.starts_with(std::path::Path::new(configured_temp.as_os_str()))
        || path.as_os_str().as_encoded_bytes().len() >= 108
    {
        return Err(OracleFailure::SocketPathTooLong);
    }
    let mut server = IpcServer::bind(&path).map_err(|_| OracleFailure::Bind)?;
    let _cleanup = CleanupPath(&path);
    let result = run_bound_oracle(&mut server, &path, &original);
    drop(server);
    result
}
fn run_bound_oracle(
    server: &mut IpcServer,
    path: &std::path::Path,
    original: &[u8],
) -> Result<(), OracleFailure> {
    let mut peer = UnixStream::connect(path).map_err(|_| OracleFailure::Connect)?;
    server.accept_client().map_err(|_| OracleFailure::Accept)?;
    let deadline = Instant::now()
        .checked_add(DEADLINE)
        .ok_or(OracleFailure::Arithmetic)?;
    let mut received = Vec::new();
    received
        .try_reserve_exact(PAYLOAD_BYTES)
        .map_err(|_| OracleFailure::Allocation)?;
    let prefix_len = queue_and_write(server, original)?;
    read_exact_prefix(&mut peer, &mut received, prefix_len, deadline)?;
    verify_prefix_and_suffix(server, original, prefix_len, &received)?;
    drain_pending(server, &mut peer, &mut received, deadline)?;
    compare_bytes(&received, &original, OracleFailure::FinalStreamMismatch)?;
    verify_open_without_extra_data(&mut peer)
}
fn encoded_blocks() -> Result<Vec<u8>, OracleFailure> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(PAYLOAD_BYTES)
        .map_err(|_| OracleFailure::Allocation)?;
    for value in 0..BLOCK_COUNT {
        let block = u32::try_from(value).map_err(|_| OracleFailure::Arithmetic)?;
        bytes.extend_from_slice(&block.to_le_bytes());
    }
    if bytes.len() != PAYLOAD_BYTES {
        return Err(OracleFailure::Arithmetic);
    }
    Ok(bytes)
}
fn queue_and_write(server: &mut IpcServer, original: &[u8]) -> Result<usize, OracleFailure> {
    queue_original(server, original)?;
    measure_initial_progress(server, original.len())
}
fn queue_original(server: &mut IpcServer, original: &[u8]) -> Result<(), OracleFailure> {
    let buffer = server
        .client_write_buffer_mut(1)
        .ok_or(OracleFailure::MissingClient)?;
    if !buffer.is_empty() {
        return Err(OracleFailure::DirtyQueue);
    }
    buffer
        .try_reserve_exact(original.len())
        .map_err(|_| OracleFailure::Allocation)?;
    buffer.extend_from_slice(original);
    Ok(())
}
fn measure_initial_progress(
    server: &mut IpcServer,
    original_len: usize,
) -> Result<usize, OracleFailure> {
    match server
        .handle_writable(1)
        .map_err(|_| OracleFailure::Handler)?
    {
        false => {}
        true => return Err(OracleFailure::ConnectionClosed),
    }
    let pending_len = queue_len(server)?;
    let written = original_len
        .checked_sub(pending_len)
        .ok_or(OracleFailure::Arithmetic)?;
    if written == 0 {
        return Err(OracleFailure::NoPositiveProgress);
    }
    if pending_len == 0 {
        return Err(OracleFailure::FullWrite);
    }
    Ok(written)
}
fn verify_prefix_and_suffix(
    server: &mut IpcServer,
    original: &[u8],
    prefix_len: usize,
    received: &[u8],
) -> Result<(), OracleFailure> {
    let expected_prefix = original
        .get(..prefix_len)
        .ok_or(OracleFailure::SliceBounds)?;
    compare_bytes(received, expected_prefix, OracleFailure::PrefixMismatch)?;
    let expected_suffix = original
        .get(prefix_len..)
        .ok_or(OracleFailure::SliceBounds)?;
    let pending = server
        .client_write_buffer_mut(1)
        .ok_or(OracleFailure::MissingClient)?;
    compare_bytes(pending, expected_suffix, OracleFailure::SuffixMismatch)
}
fn read_exact_prefix(
    peer: &mut UnixStream,
    received: &mut Vec<u8>,
    count: usize,
    deadline: Instant,
) -> Result<(), OracleFailure> {
    let target = received
        .len()
        .checked_add(count)
        .ok_or(OracleFailure::Arithmetic)?;
    let mut scratch = [0_u8; READ_BUFFER_BYTES];
    for _ in 0..MAX_READS_PER_TURN {
        if received.len() == target {
            return Ok(());
        }
        check_deadline(deadline)?;
        read_prefix_chunk(peer, received, target, &mut scratch)?;
    }
    Err(OracleFailure::ReadLimit)
}
fn read_prefix_chunk(
    peer: &mut UnixStream,
    received: &mut Vec<u8>,
    target: usize,
    scratch: &mut [u8],
) -> Result<(), OracleFailure> {
    let remaining = target
        .checked_sub(received.len())
        .ok_or(OracleFailure::Arithmetic)?;
    let window = remaining.min(scratch.len());
    let writable = scratch
        .get_mut(..window)
        .ok_or(OracleFailure::SliceBounds)?;
    match peer.read(writable) {
        Ok(0) => Err(OracleFailure::PeerEof),
        Ok(count) => append_read(received, scratch, count),
        Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
        Err(_) => Err(OracleFailure::PeerRead),
    }
}
fn drain_pending(
    server: &mut IpcServer,
    peer: &mut UnixStream,
    received: &mut Vec<u8>,
    deadline: Instant,
) -> Result<(), OracleFailure> {
    for _ in 0..MAX_DRAIN_TURNS {
        check_deadline(deadline)?;
        if queue_len(server)? == 0 {
            return Ok(());
        }
        drain_turn(server, peer, received, deadline)?;
    }
    if queue_len(server)? == 0 {
        Ok(())
    } else {
        Err(OracleFailure::DrainLimit)
    }
}
fn drain_turn(
    server: &mut IpcServer,
    peer: &mut UnixStream,
    received: &mut Vec<u8>,
    deadline: Instant,
) -> Result<(), OracleFailure> {
    let pending_before = queue_len(server)?;
    match server
        .handle_writable(1)
        .map_err(|_| OracleFailure::Handler)?
    {
        false => {}
        true => return Err(OracleFailure::ConnectionClosed),
    }
    pending_before
        .checked_sub(queue_len(server)?)
        .ok_or(OracleFailure::Arithmetic)?;
    drain_available(peer, received, deadline)
}
fn drain_available(
    peer: &mut UnixStream,
    received: &mut Vec<u8>,
    deadline: Instant,
) -> Result<(), OracleFailure> {
    let mut scratch = [0_u8; READ_BUFFER_BYTES];
    for _ in 0..MAX_READS_PER_TURN {
        check_deadline(deadline)?;
        let remaining = PAYLOAD_BYTES
            .checked_sub(received.len())
            .ok_or(OracleFailure::Arithmetic)?;
        if remaining == 0 {
            return Ok(());
        }
        let window = remaining.min(scratch.len());
        let writable = scratch
            .get_mut(..window)
            .ok_or(OracleFailure::SliceBounds)?;
        match peer.read(writable) {
            Ok(0) => return Err(OracleFailure::PeerEof),
            Ok(count) => append_read(received, &scratch, count)?,
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(_) => return Err(OracleFailure::PeerRead),
        }
    }
    Err(OracleFailure::ReadLimit)
}
fn append_read(received: &mut Vec<u8>, scratch: &[u8], count: usize) -> Result<(), OracleFailure> {
    let bytes = scratch.get(..count).ok_or(OracleFailure::SliceBounds)?;
    received
        .len()
        .checked_add(bytes.len())
        .filter(|length| *length <= PAYLOAD_BYTES)
        .ok_or(OracleFailure::Arithmetic)?;
    received.extend_from_slice(bytes);
    Ok(())
}
fn queue_len(server: &mut IpcServer) -> Result<usize, OracleFailure> {
    server
        .client_write_buffer_mut(1)
        .map(|buffer| buffer.len())
        .ok_or(OracleFailure::MissingClient)
}
fn compare_bytes(
    actual: &[u8],
    expected: &[u8],
    failure: OracleFailure,
) -> Result<(), OracleFailure> {
    if actual == expected {
        Ok(())
    } else {
        Err(failure)
    }
}
fn check_deadline(deadline: Instant) -> Result<(), OracleFailure> {
    if Instant::now() < deadline {
        Ok(())
    } else {
        Err(OracleFailure::Deadline)
    }
}
fn verify_open_without_extra_data(peer: &mut UnixStream) -> Result<(), OracleFailure> {
    let mut extra = [0_u8; 1];
    match peer.read(&mut extra) {
        Ok(0) => Err(OracleFailure::PeerEof),
        Ok(_) => Err(OracleFailure::ExtraData),
        Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
        Err(_) => Err(OracleFailure::FinalRead),
    }
}
