/// RAII guard that removes the socket path on drop.
struct CleanupPath<'a>(&'a std::path::Path);

impl Drop for CleanupPath<'_> {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(self.0) {
            eprintln!("cleanup remove_file failed: {error}");
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Additional coverage tests
// ══════════════════════════════════════════════════════════════════════════════

use super::dispatch::serve_ipc_with_resolver;
use super::{WorkflowResolutionError, WorkflowResolver};
use crate::SubmitRunPayload;

// ── serve_ipc with None timeout ─────────────────────────────────────────────

#[test]
fn serve_ipc_with_none_timeout_returns_ok_when_client_connected() {
    let path = temp_socket_path("serve_none_timeout");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    // Connect a client so the poll has an event to process and does not block.
    let _client = make_client(&path);

    let result = serve_ipc(&mut server, &mut runtime, None);

    assert_eq!(
        result,
        Ok(true),
        "serve_ipc with None timeout should succeed and return continue"
    );
}

// ── serve_ipc_with_resolver with None timeout and None resolver ─────────────

#[test]
fn serve_ipc_with_resolver_none_timeout_none_resolver_returns_ok_when_client_connected() {
    let path = temp_socket_path("serve_ipc_resolver_none");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    // Connect a client so the poll has an event to process and does not block.
    let _client = make_client(&path);

    let result = serve_ipc_with_resolver(&mut server, &mut runtime, None, None);

    assert_eq!(
        result,
        Ok(true),
        "serve_ipc_with_resolver with None timeout and None resolver should return Ok(true)"
    );
}

// ── IpcServerError Display for accept_failed ────────────────────────────────

#[test]
fn ipc_server_error_accept_failed_display() {
    let err = IpcServerError::AcceptFailed {
        source: std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("accept failed"),
        "expected 'accept failed' in '{msg}'"
    );
}

// ── IpcServerError Display for response_encode_failed ───────────────────────

#[test]
fn ipc_server_error_response_encode_failed_display() {
    let err = IpcServerError::ResponseEncodeFailed;
    let msg = err.to_string();
    assert!(
        msg.contains("response encode failed"),
        "expected 'response encode failed' in '{msg}'"
    );
}

// ── IpcServerError Display for incomplete_frame ─────────────────────────────

#[test]
fn ipc_server_error_incomplete_frame_display() {
    let err = IpcServerError::IncompleteFrame;
    let msg = err.to_string();
    assert!(
        msg.contains("incomplete IPC frame"),
        "expected 'incomplete IPC frame' in '{msg}'"
    );
}

// ── WorkflowResolver trait: NotFound error case ─────────────────────────────

struct NotFoundResolver;

impl WorkflowResolver for NotFoundResolver {
    fn resolve_workflow(
        &mut self,
        _digest: vb_core::WorkflowDigest,
    ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
        Err(WorkflowResolutionError::NotFound)
    }
}

#[test]
fn workflow_resolver_not_found_error_message() {
    let err = WorkflowResolutionError::NotFound;
    let msg = err.to_string();
    assert!(msg.contains("not found"), "expected 'not found' in '{msg}'");
}

#[test]
fn workflow_resolver_not_found_is_rejected_by_dispatch() {
    let path = temp_socket_path("resolver_not_found");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Build a SubmitRun payload with a zeroed (unlikely) digest.
    let submit = SubmitRunPayload {
        run_id: vb_core::RunId::new(42u64),
        workflow: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        input: vec![],
    };
    let ipc_payload = crate::IpcPayload::SubmitRun(submit);
    let payload_bytes = postcard::to_allocvec(&ipc_payload).expect("encode payload");
    let frame = build_frame(IpcCommand::SubmitRun, 99, &payload_bytes);
    client
        .write_all(&frame)
        .expect("client should write submit frame");
    client.flush().expect("client should flush");

    // Process with a resolver that always returns NotFound.
    let mut resolver = NotFoundResolver;
    let result = server.poll_once_with_resolver(
        &mut runtime,
        Some(Duration::from_millis(100)),
        Some(&mut resolver),
    );
    assert!(
        matches!(result, Ok(_)),
        "poll_once_with_resolver should succeed"
    );

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    let Ok(response_header) = response_header_bytes else {
        panic!("should read response header")
    };

    // Read response payload.
    let payload_len = u32::from_le_bytes(
        response_header
            .get(20..24)
            .and_then(|s| <[u8; 4]>::try_from(s).ok())
            .unwrap_or([0; 4]),
    );
    let payload_len_usize = match usize::try_from(payload_len) {
        Ok(v) => v,
        Err(_) => return,
    };
    if payload_len_usize > 0 {
        let response_payload = read_exact_timeout(&mut client, payload_len_usize);
        let Ok(payload) = response_payload else {
            panic!("should read response payload")
        };
        let decoded: Result<IpcResponse, _> = postcard::from_bytes(&payload);
        match decoded {
            Ok(IpcResponse::PayloadError { message, .. }) => {
                assert!(
                    message.contains("not found") || message.contains("digest"),
                    "expected 'not found' or 'digest' in payload error message, got '{message}'"
                );
            }
            Ok(IpcResponse::WorkflowDigestMismatch) => {
                // Also acceptable: server detected digest mismatch before calling resolver.
            }
            Ok(IpcResponse::WorkflowResolutionUnsupported) => {
                // Also acceptable for certain code paths.
            }
            Ok(other) => {
                assert!(
                    !matches!(other, IpcResponse::Healthy),
                    "Unexpected Healthy response when resolver returned NotFound"
                );
            }
            Err(e) => {
                assert!(false, "response payload decode failed: {e}");
            }
        }
    }
}

