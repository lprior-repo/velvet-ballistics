use super::*;
use crate::IpcCommand;

// ── append_read_bytes tests ──

#[test]
fn append_read_bytes_appends_data_to_empty_buffer() {
    let mut read_buffer = Vec::new();
    let temp_buf = [0xAB_u8; 4096];
    let result = append_read_bytes(&mut read_buffer, &temp_buf, 10);
    assert_eq!(result, Ok(()), "appending 10 bytes should succeed");
    assert_eq!(read_buffer.len(), 10);
    assert_eq!(read_buffer.as_slice(), &[0xAB; 10]);
}

#[test]
fn append_read_bytes_appends_to_existing_buffer() {
    let mut read_buffer = vec![1, 2, 3];
    let temp_buf = [4u8; 4096];
    let result = append_read_bytes(&mut read_buffer, &temp_buf, 3);
    let Ok(()) = result else {
        panic!("appending 3 bytes should succeed");
    };
    assert_eq!(read_buffer.len(), 6);
    assert_eq!(read_buffer.as_slice(), &[1, 2, 3, 4, 4, 4]);
}

#[test]
fn append_read_bytes_with_zero_bytes_read() {
    let mut read_buffer = Vec::new();
    let temp_buf = [0u8; 4096];
    let result = append_read_bytes(&mut read_buffer, &temp_buf, 0);
    let Ok(()) = result else {
        panic!("zero bytes should succeed");
    };
    assert!(read_buffer.is_empty());
}

#[test]
fn append_read_bytes_rejects_bytes_read_exceeding_temp_buf() {
    let mut read_buffer = Vec::new();
    let temp_buf = [0u8; 4096];
    // bytes_read > 4096 is impossible in practice but tests the guard
    let result = append_read_bytes(&mut read_buffer, &temp_buf, 5000);
    let Err(e) = result else {
        panic!("bytes_read > temp_buf size should fail");
    };
    assert!(
        matches!(e, IpcServerError::FrameInvalid { .. }),
        "expected FrameInvalid, got {e:?}"
    );
}

// ── read_buffer_header tests ──

#[test]
fn read_buffer_header_returns_incomplete_frame_for_short_buffer() {
    let short_buf = vec![0u8; 10];
    let result = read_buffer_header(&short_buf);
    let Err(err) = result else {
        panic!("short buffer should fail");
    };
    let msg = err.to_string();
    assert!(
        msg.contains("incomplete"),
        "expected 'incomplete' in '{msg}'"
    );
}

#[test]
fn read_buffer_header_returns_incomplete_frame_for_empty_buffer() {
    let empty_buf: Vec<u8> = Vec::new();
    let result = read_buffer_header(&empty_buf);
    let Err(e) = result else {
        panic!("expected IncompleteFrame for empty buffer");
    };
    assert!(
        matches!(e, IpcServerError::IncompleteFrame),
        "expected IncompleteFrame, got {e:?}"
    );
}

#[test]
fn read_buffer_header_succeeds_with_exact_header_length() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let encoded = header.encode();
    let Ok(encoded) = encoded else {
        panic!("header should encode");
    };
    let buf = encoded.to_vec();
    let result = read_buffer_header(&buf);
    let Ok(_) = result else {
        panic!("exact header length should succeed");
    };
}

#[test]
fn read_buffer_header_succeeds_with_extra_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let encoded = header.encode();
    let Ok(encoded) = encoded else {
        panic!("header should encode");
    };
    let mut buf = encoded.to_vec();
    buf.extend_from_slice(&[0xFF; 100]); // extra payload bytes
    let result = read_buffer_header(&buf);
    let Ok(_) = result else {
        panic!("extra bytes after header should still succeed");
    };
}

// ── frame_total_len tests ──

#[test]
fn frame_total_len_header_only_zero_payload() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let result = frame_total_len(&header);
    let Ok(val) = result else {
        panic!("frame_total_len should succeed");
    };
    assert_eq!(val, IPC_HEADER_LEN);
}

#[test]
fn frame_total_len_with_payload() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let result = frame_total_len(&header);
    let Ok(val) = result else {
        panic!("frame_total_len should succeed");
    };
    assert_eq!(val, IPC_HEADER_LEN + 100);
}

#[test]
fn frame_total_len_with_max_reasonable_payload() {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, 1000);
    let result = frame_total_len(&header);
    let Ok(val) = result else {
        panic!("frame_total_len should succeed");
    };
    assert_eq!(val, IPC_HEADER_LEN + 1000);
}

// ── extract_payload tests ──

#[test]
fn extract_payload_returns_incomplete_when_buffer_too_short() {
    let mut read_buffer = vec![0u8; 10];
    let result = extract_payload(&mut read_buffer, 50);
    let Err(e) = result else {
        panic!("expected IncompleteFrame for short buffer");
    };
    assert!(
        matches!(e, IpcServerError::IncompleteFrame),
        "expected IncompleteFrame, got {e:?}"
    );
}

#[test]
fn extract_payload_extracts_header_plus_payload() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode();
    let Ok(encoded) = encoded else {
        panic!("header should encode");
    };
    let mut read_buffer = encoded.to_vec();
    read_buffer.extend_from_slice(b"test");
    let total_len = IPC_HEADER_LEN + 4;

    let result = extract_payload(&mut read_buffer, total_len);
    let Ok(payload) = result else {
        panic!("extract should succeed");
    };
    assert_eq!(payload.as_slice(), b"test");
}

#[test]
fn extract_payload_preserves_remaining_bytes_in_buffer() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode();
    let Ok(encoded) = encoded else {
        panic!("header should encode");
    };
    let mut read_buffer = encoded.to_vec();
    read_buffer.extend_from_slice(b"test");
    read_buffer.extend_from_slice(b"extra");
    let total_len = IPC_HEADER_LEN + 4;

    let result = extract_payload(&mut read_buffer, total_len);
    let Ok(_) = result else {
        panic!("extract should succeed");
    };
    assert_eq!(
        read_buffer.as_slice(),
        b"extra",
        "remaining bytes should stay in buffer"
    );
}

#[test]
fn extract_payload_returns_empty_for_zero_payload_len() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let encoded = header.encode();
    let Ok(encoded) = encoded else {
        panic!("header should encode");
    };
    let mut read_buffer = encoded.to_vec();
    let result = extract_payload(&mut read_buffer, IPC_HEADER_LEN);
    let Ok(payload) = result else {
        panic!("extract should succeed");
    };
    assert!(payload.is_empty());
}

// ── frame_error_response tests ──

#[test]
fn frame_error_response_produces_frame_error_variant() {
    let err = IpcError::InvalidMagic { actual: 0xDEAD };
    let response = frame_error_response(err);
    let IpcResponse::FrameError { message } = response else {
        return;
    };
    assert!(message.contains("magic"), "expected 'magic' in '{message}'");
}

#[test]
fn frame_error_response_includes_display_of_source() {
    let err = IpcError::PayloadTooLarge {
        actual: 999,
        limit: 10,
    };
    let response = frame_error_response(err);
    let IpcResponse::FrameError { message } = response else {
        return;
    };
    assert!(
        message.contains("999"),
        "expected actual value in '{message}'"
    );
    assert!(
        message.contains("10"),
        "expected limit value in '{message}'"
    );
}

// ── borrow_workflow_resolver tests ──

#[test]
fn borrow_workflow_resolver_returns_none_for_none_outer() {
    let mut outer: Option<&mut dyn crate::server::WorkflowResolver> = None;
    let result = borrow_workflow_resolver(&mut outer);
    assert!(result.is_none());
}

#[test]
fn borrow_workflow_resolver_returns_some_for_some_outer() {
    struct DummyResolver;
    impl crate::server::WorkflowResolver for DummyResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, crate::server::WorkflowResolutionError>
        {
            Err(crate::server::WorkflowResolutionError::NotFound)
        }
    }
    let mut resolver = DummyResolver;
    let mut outer: Option<&mut dyn crate::server::WorkflowResolver> = Some(&mut resolver);
    let result = borrow_workflow_resolver(&mut outer);
    assert!(result.is_some());
}

// ── append_read_bytes overflow / bounds tests ──

#[test]
fn append_read_bytes_accepts_exactly_at_max() {
    let max = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
    let mut read_buffer = vec![0u8; max - 1];
    let temp_buf = [1u8; 4096];
    let result = append_read_bytes(&mut read_buffer, &temp_buf, 1);
    assert_eq!(result, Ok(()), "expected Ok when appending exactly to max");
    assert_eq!(read_buffer.len(), max);
}

#[test]
fn append_read_bytes_rejects_when_buffer_would_exceed_max() {
    let max = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
    let mut read_buffer = vec![0u8; max - 5];
    let temp_buf = [1u8; 4096];
    let result = append_read_bytes(&mut read_buffer, &temp_buf, 10);
    assert!(
        matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
        "expected ReadBufferTooLarge when max exceeded"
    );
}

#[test]
fn append_read_bytes_rejects_at_exactly_one_over_max() {
    let max = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
    let mut read_buffer = vec![0u8; max];
    let temp_buf = [1u8; 4096];
    let result = append_read_bytes(&mut read_buffer, &temp_buf, 1);
    assert!(
        matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
        "expected ReadBufferTooLarge when exactly one over max"
    );
}

#[test]
fn append_read_bytes_checked_add_overflow_returns_too_large() {
    let result = append_read_bytes_checked_add(usize::MAX, 1);
    assert!(
        matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
        "expected ReadBufferTooLarge on usize overflow"
    );
}

// ── frame_total_len overflow test ──

#[test]
fn frame_total_len_checked_add_overflow_returns_too_large() {
    let result = frame_total_len_checked_add(usize::MAX, 1);
    assert!(
        matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
        "expected ReadBufferTooLarge on usize overflow"
    );
}

// ── extract_payload exact-length test ──

#[test]
fn extract_payload_exact_length_clears_buffer() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode().expect("encode");
    let mut read_buffer = encoded.to_vec();
    read_buffer.extend_from_slice(b"test");
    let total_len = IPC_HEADER_LEN + 4;

    let result = extract_payload(&mut read_buffer, total_len);
    let Ok(payload) = result else {
        panic!("extract should succeed");
    };
    assert_eq!(payload.as_slice(), b"test");
    assert!(
        read_buffer.is_empty(),
        "read_buffer should be empty after exact extraction"
    );
}

// ── send_response error-path tests ──

use mio::Poll;

struct ResetOnDrop;
impl Drop for ResetOnDrop {
    fn drop(&mut self) {
        test_hooks::FORCE_POSTCARD_FAIL.set(false);
        test_hooks::FORCE_HEADER_ENCODE_FAIL.set(false);
        test_hooks::FORCE_FLUSH_FAIL.set(false);
    }
}

fn setup_mio_stream_with_peer() -> (
    mio::net::UnixStream,
    std::os::unix::net::UnixStream,
    Poll,
    mio::Token,
) {
    let (std_a, std_b) = std::os::unix::net::UnixStream::pair().unwrap();
    std_a.set_nonblocking(true).unwrap();
    std_b.set_nonblocking(true).unwrap();
    let mut stream = mio::net::UnixStream::from_std(std_a);
    let poll = Poll::new().unwrap();
    let token = mio::Token(1);
    poll.registry()
        .register(&mut stream, token, mio::Interest::READABLE)
        .unwrap();
    (stream, std_b, poll, token)
}

#[test]
fn send_response_reregisters_when_written_zero_and_buffer_not_empty() {
    let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
    let registry = poll.registry().try_clone().unwrap();

    // Fill the stream's send buffer so the next write returns WouldBlock.
    let buf = [0u8; 4096];
    loop {
        match stream.write(&buf) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    let mut write_buffer = Vec::new();
    let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let response = crate::server::IpcResponse::Healthy;

    let result = send_response(
        &mut stream,
        &mut write_buffer,
        &registry,
        token,
        &request_header,
        &response,
    );

    let Ok(()) = result else {
        panic!(
            "send_response should succeed after reregister: {:?}",
            result.err()
        );
    };
    assert!(
        !write_buffer.is_empty(),
        "write_buffer should still contain data after WouldBlock"
    );
}

#[test]
fn send_response_returns_error_when_postcard_encode_fails() {
    let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
    let registry = poll.registry().try_clone().unwrap();

    test_hooks::FORCE_POSTCARD_FAIL.set(true);
    let _guard = ResetOnDrop;

    let mut write_buffer = Vec::new();
    let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let response = crate::server::IpcResponse::Healthy;

    let result = send_response(
        &mut stream,
        &mut write_buffer,
        &registry,
        token,
        &request_header,
        &response,
    );

    let Err(e) = result else {
        panic!("expected ResponseEncodeFailed");
    };
    assert!(
        matches!(e, IpcServerError::ResponseEncodeFailed),
        "expected ResponseEncodeFailed, got {e:?}"
    );
}

#[test]
fn send_response_returns_error_when_header_encode_fails() {
    let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
    let registry = poll.registry().try_clone().unwrap();

    test_hooks::FORCE_HEADER_ENCODE_FAIL.set(true);
    let _guard = ResetOnDrop;

    let mut write_buffer = Vec::new();
    let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let response = crate::server::IpcResponse::Healthy;

    let result = send_response(
        &mut stream,
        &mut write_buffer,
        &registry,
        token,
        &request_header,
        &response,
    );

    let Err(e) = result else {
        panic!("expected ResponseEncodeFailed");
    };
    assert!(
        matches!(e, IpcServerError::ResponseEncodeFailed),
        "expected ResponseEncodeFailed, got {e:?}"
    );
}

#[test]
fn send_response_returns_error_when_flush_fails_non_wouldblock() {
    let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
    let registry = poll.registry().try_clone().unwrap();

    test_hooks::FORCE_FLUSH_FAIL.set(true);
    let _guard = ResetOnDrop;

    let mut write_buffer = Vec::new();
    let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let response = crate::server::IpcResponse::Healthy;

    let result = send_response(
        &mut stream,
        &mut write_buffer,
        &registry,
        token,
        &request_header,
        &response,
    );

    let Err(e) = result else {
        panic!("expected ResponseWriteFailed");
    };
    assert!(
        matches!(e, IpcServerError::ResponseWriteFailed { .. }),
        "expected ResponseWriteFailed, got {e:?}"
    );
}

#[test]
fn send_response_returns_error_when_write_fails_broken_pipe() {
    let (mut stream, peer, poll, token) = setup_mio_stream_with_peer();
    let registry = poll.registry().try_clone().unwrap();
    drop(peer);

    let mut write_buffer = Vec::new();
    let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let response = crate::server::IpcResponse::Healthy;

    let result = send_response(
        &mut stream,
        &mut write_buffer,
        &registry,
        token,
        &request_header,
        &response,
    );

    let Err(e) = result else {
        panic!("expected ResponseWriteFailed after dropping peer");
    };
    assert!(
        matches!(e, IpcServerError::ResponseWriteFailed { .. }),
        "expected ResponseWriteFailed, got {e:?}"
    );
}
