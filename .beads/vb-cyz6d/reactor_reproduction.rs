#![forbid(unsafe_code)]
//! Bounded audit reproductions against public production APIs; no copied reactor.
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::os::unix::net::UnixStream;
use std::time::Duration;
use vb_ipc::server::{IpcResponse, IpcServer};
use vb_ipc::{IpcCommand, IpcFrameHeader, MaxPayloadBytes};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

type AuditResult<T> = Result<T, Box<dyn std::error::Error>>;

fn runtime() -> Runtime {
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    Runtime::new_for_tests_and_benchmarks_only(NonZeroUsize::MIN, config)
}

fn frame(payload: usize) -> AuditResult<Vec<u8>> {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 7, u32::try_from(payload)?);
    let mut bytes = header.encode()?.to_vec();
    bytes.resize(bytes.len().checked_add(payload).ok_or("frame overflow")?, 0);
    Ok(bytes)
}

fn poll(server: &mut IpcServer, runtime: &mut Runtime, count: usize) -> AuditResult<()> {
    for _ in 0..count {
        server.poll_once(runtime, Some(Duration::from_millis(1)))?;
    }
    Ok(())
}

fn socket_path(case: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/tmp/vb_cyz6d_{case}_{}.sock", std::process::id()))
}

fn large_read() -> AuditResult<()> {
    let path = socket_path("large");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut client = UnixStream::connect(&path)?;
    poll(&mut server, &mut runtime, 1)?;
    client.write_all(&frame(8192)?)?;
    poll(&mut server, &mut runtime, 20)?;
    client.set_nonblocking(true)?;
    let mut response = [0_u8; 64];
    println!("large_read: complete 8216-byte frame, 20 polling turns, response={:?}", client.read(&mut response));
    drop(server);
    std::fs::remove_file(path)?;
    Ok(())
}

fn accept_burst() -> AuditResult<()> {
    let path = socket_path("burst");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut clients = Vec::new();
    for _ in 0..3 {
        let mut client = UnixStream::connect(&path)?;
        client.write_all(&frame(0)?)?;
        clients.push(client);
    }
    poll(&mut server, &mut runtime, 20)?;
    for (index, client) in clients.iter_mut().enumerate() {
        client.set_nonblocking(true)?;
        let mut response = [0_u8; 64];
        println!("accept_burst: client={index}, response={:?}", client.read(&mut response));
    }
    drop(server);
    std::fs::remove_file(path)?;
    Ok(())
}

fn overloaded_accept() -> AuditResult<()> {
    let path = socket_path("full");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut clients = Vec::new();
    for _ in 0..256 {
        clients.push(UnixStream::connect(&path)?);
        poll(&mut server, &mut runtime, 1)?;
    }
    clients.push(UnixStream::connect(&path)?);
    println!("overloaded_accept: accepted 256, next poll={:?}", server.poll_once(&mut runtime, Some(Duration::from_millis(1))));
    drop(server);
    std::fs::remove_file(path)?;
    Ok(())
}

fn overwrite_response() -> AuditResult<()> {
    let (writer, _reader) = UnixStream::pair()?;
    writer.set_nonblocking(true)?;
    let mut writer = mio::net::UnixStream::from_std(writer);
    let poll = mio::Poll::new()?;
    poll.registry().register(&mut writer, mio::Token(1), mio::Interest::READABLE)?;
    let filler = [0_u8; 65_536];
    let mut blocked = false;
    for _ in 0..512 {
        match writer.write(&filler) {
            Ok(_) => {},
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => { blocked = true; break; },
            Err(error) => return Err(error.into()),
        }
    }
    if !blocked { return Err("could not create socket backpressure within budget".into()); }
    let mut bytes = Vec::new();
    for correlation in [11, 22] {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, correlation, 0);
        vb_ipc::server::send_response(&mut writer, &mut bytes, poll.registry(), mio::Token(1), &header, &IpcResponse::Healthy)?;
        let prefix = bytes.get(..vb_ipc::IPC_HEADER_LEN).ok_or("missing queued header")?;
        let header_bytes = <[u8; vb_ipc::IPC_HEADER_LEN]>::try_from(prefix)?;
        let pending = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT)?;
        println!("overwrite_response: sent correlation={correlation}, pending correlation={}, pending bytes={}", pending.correlation, bytes.len());
    }
    Ok(())
}

fn main() -> AuditResult<()> {
    large_read()?;
    accept_burst()?;
    overloaded_accept()?;
    overwrite_response()
}
