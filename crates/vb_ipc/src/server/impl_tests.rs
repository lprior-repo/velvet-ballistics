#![forbid(unsafe_code)]
//! Tests for `IpcServer` implementation (`impl_.rs`).
//!
//! Covers:
//! - `IpcServer::bind` — socket binding and cleanup
//! - `IpcServer::poll_once` / `poll_once_with_resolver` — event polling
//! - `serve_ipc` — public dispatch function
//! - Private helpers via integration: `accept_client`, `handle_readable`,
//!   `handle_writable`, `remove_client`

use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use mio::net::UnixStream;

use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

use super::error::IpcServerError;
use super::{IpcResponse, IpcServer, serve_ipc};
use crate::IPC_HEADER_LEN;
use crate::IPC_MAGIC;
use crate::IPC_VERSION;
use crate::IpcCommand;
use crate::IpcFrameHeader;
use crate::IpcPayload;
use crate::MaxPayloadBytes;
use vb_core::{RunId, WorkflowDigest};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn temp_socket_path(name: &str) -> PathBuf {
    // Per-test uniqueness is guaranteed by the process id (parallel cargo
    // test workers) and a static monotonic counter (successive invocations
    // within the same process). Each test owns cleanup of its socket path.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let name = name.chars().take(8).collect::<String>();
    dir.join(format!("vb_impl_{name}_{pid}_{counter:04x}.sock"))
}

fn make_runtime() -> Runtime {
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    Runtime::new_for_tests_and_benchmarks_only(NonZeroUsize::MIN, config)
}

fn make_client(server_path: &std::path::Path) -> UnixStream {
    UnixStream::connect(server_path).expect("test client should connect to server socket")
}

/// Builds a valid IPC frame header + payload bytes for the given command and postcard payload.
fn build_frame(command: IpcCommand, correlation: u64, payload_bytes: &[u8]) -> Vec<u8> {
    let header = IpcFrameHeader::new(
        command,
        0,
        correlation,
        match u32::try_from(payload_bytes.len()) {
            Ok(v) => v,
            Err(_) => 0,
        },
    );
    let Ok(encoded) = header.encode() else {
        return Vec::new();
    };
    let mut frame = encoded.to_vec();
    frame.extend_from_slice(payload_bytes);
    frame
}

/// Reads exactly `n` bytes from the stream with a short timeout.
fn read_exact_timeout(stream: &mut dyn Read, n: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut buf = vec![0u8; n];
    let mut read_total = 0usize;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while read_total < n {
        if std::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "response deadline exceeded",
            ));
        }
        match stream.read(&mut buf[read_total..]) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "eof",
                ));
            }
            Ok(count) => read_total = read_total.saturating_add(count),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::yield_now();
            }
            Err(e) => return Err(e),
        }
    }
    Ok(buf)
}

include!("impl_tests/tests_bind_poll.rs");

include!("impl_tests/tests_dispatch.rs");

include!("impl_tests/tests_frames.rs");

include!("impl_tests/tests_resolver.rs");

include!("impl_tests/tests_roundtrip.rs");

include!("impl_tests/tests_handlers.rs");

include!("impl_tests/tests_edges.rs");

#[test]
fn ipc_socket_path_uses_configured_temp_dir() {
    let path = temp_socket_path("configured");
    let base = std::env::temp_dir();
    assert_eq!(path.parent(), Some(base.as_path()));
    assert!(path.as_os_str().as_encoded_bytes().len() < 108);
}
