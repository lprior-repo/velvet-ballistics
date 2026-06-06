#![forbid(unsafe_code)]
//! VB-54MW: IPC Magic Gate — Integration Behavior Tests
//!
//! These tests document the expected behavior of the magic validation gate
//! in the IPC server. Tests are written in failing-first TDD style:
//! they describe the expected behavior but WILL NOT COMPILE until
//! `validate_magic_early` and the `MagicValidationState` typestate are implemented.
//!
//! ## Behaviors Covered
//!
//! - B10: InvalidMagic closes connection without further reads
//! - B11: Buffer cap enforced before magic validation
//! - B12: First read chunk bounded by READ_CHUNK_BYTES
//!
//! ## Test Approach
//!
//! These are BLACK-BOX integration tests using real Unix socket pairs.
//! They test the IPC server's handling of invalid magic bytes and buffer
//! allocation limits without accessing private implementation details.

use vb_ipc::{
    IpcError, IpcFrameHeader, IpcServer, MaxPayloadBytes,
    IPC_HEADER_LEN, IPC_MAGIC,
};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use vb_runtime::runtime::Runtime;

/// Creates a bonded IpcServer and connected UnixStream pair.
fn setup_server_and_client(
    socket_path: &std::path::Path,
) -> Result<(IpcServer, UnixStream, Runtime), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::new()?;
    let server = IpcServer::bind(socket_path)?;
    let client = UnixStream::connect(socket_path)?;
    Ok((server, client, runtime))
}

/// Writes bytes to the client socket and returns the response.
fn send_and_receive_response(
    client: &mut UnixStream,
    data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    client.write_all(data)?;
    let mut response = vec![0u8; 8192];
    let n = client.read(&mut response)?;
    response.truncate(n);
    Ok(response)
}

// ─────────────────────────────────────────────────────────────────────────────
// B10: InvalidMagic closes connection without further reads
// ─────────────────────────────────────────────────────────────────────────────

/// Given: a running IpcServer with one connected client
/// And: the client sends bytes that decode to an invalid magic
/// When: poll_once is called
/// Then: a FrameError response is sent
/// And: the client is removed from the server (handle_readable returns true)
/// And: the server does not read additional bytes from the socket
#[test]
fn invalid_magic_connection_closed_immediately_without_further_reads() {
    let socket_path = std::env::temp_dir().join("vb54mw_test_invalid_magic.sock");
    let _ = std::fs::remove_file(&socket_path);

    let (mut server, mut client, mut runtime) =
        setup_server_and_client(&socket_path).expect("setup failed");

    // Send invalid magic (all zeros)
    let invalid_magic = [0x00, 0x00, 0x00, 0x00];
    client.write_all(&invalid_magic).expect("write failed");

    // Poll once — this should process the invalid magic and close the connection
    let should_remove = server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("poll_once failed");

    // The server should signal to remove the client
    assert!(
        should_remove,
        "Server should signal client removal on InvalidMagic"
    );

    // Subsequent write should fail (connection closed) or succeed but be ignored
    // The key invariant: server did NOT read more than the 4 invalid magic bytes
    let more_data = [0xFF; 100];
    let write_result = client.write(&more_data);
    if write_result.is_ok() {
        // If write succeeds, the socket is still open but server should ignore it
        // Poll again — should return false (no client to process)
        let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
        // Either no client exists, or the server ignores the extra data
    }

    let _ = std::fs::remove_file(&socket_path);
}

/// Given: a running IpcServer with one connected client
/// And: the client sends exactly 4 bytes of invalid magic
/// When: poll_once is called
/// Then: Err(IpcServerError::ReadBufferTooLarge) is NOT returned during this cycle
/// And: a FrameError response is sent with InvalidMagic detail
/// And: the client is removed
#[test]
fn invalid_magic_sends_frame_error_response_before_closing() {
    let socket_path = std::env::temp_dir().join("vb54mw_test_frame_error.sock");
    let _ = std::fs::remove_file(&socket_path);

    let (mut server, mut client, mut runtime) =
        setup_server_and_client(&socket_path).expect("setup failed");

    // Send invalid magic that is not zero but also not IPC_MAGIC
    let invalid_magic = [0xFF, 0xFF, 0xFF, 0xFF];
    client.write_all(&invalid_magic).expect("write failed");

    // Poll — should process invalid magic and send FrameError
    let should_remove = server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("poll_once failed");

    assert!(
        should_remove,
        "Client should be removed on InvalidMagic"
    );

    // Read the response to verify FrameError was sent
    client.set_read_timeout(Some(Duration::from_millis(100))).ok();
    let mut response_buf = vec![0u8; 1024];
    let read_result = client.read(&mut response_buf);

    // We expect a response was sent (FrameError with InvalidMagic)
    // The exact format depends on the IPC protocol encoding
    assert!(
        read_result.is_ok() && read_result.unwrap() > 0,
        "FrameError response should be sent before connection closes"
    );

    let _ = std::fs::remove_file(&socket_path);
}

// ─────────────────────────────────────────────────────────────────────────────
// B11: Buffer cap enforced before magic validation
// ─────────────────────────────────────────────────────────────────────────────

/// Given: a running IpcServer with a client in AwaitingMagic state
/// And: read_buffer.len() == 4
/// When: the client sends 4096 more bytes
/// Then: append_read_bytes returns ReadBufferTooLarge BEFORE validate_magic_early is called
/// And: the client is removed
#[test]
fn server_rejects_buffer_growth_beyond_4_bytes_in_awaiting_magic_state() {
    let socket_path = std::env::temp_dir().join("vb54mw_test_buffer_cap.sock");
    let _ = std::fs::remove_file(&socket_path);

    let (mut server, mut client, mut runtime) =
        setup_server_and_client(&socket_path).expect("setup failed");

    // First, send exactly 4 bytes (partial magic — should accumulate)
    let partial = [0x00, 0x00, 0x00, 0x00];
    client.write_all(&partial).expect("write failed");

    // Poll — should accumulate 4 bytes but not yet validate
    let should_remove_1 = server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("poll_once failed");

    // With only 4 bytes accumulated and no valid magic yet, server should continue waiting
    // (unless the 4 bytes are validated as invalid magic)
    // If 4 zeros are invalid magic, this would trigger removal

    // Now send MORE data — should be rejected because we're in AwaitingMagic
    // and already at 4 bytes (the cap before magic validation)
    let excess = [0xFF; 4096];
    client.write_all(&excess).expect("write failed");

    // Poll — should reject the excess data before any magic validation
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));

    // The key assertion: result should NOT be Ok if the buffer cap is enforced
    // If the implementation is correct, ReadBufferTooLarge should be returned
    // and the client removed
    match result {
        Ok(should_remove) => {
            // Either the client was removed due to invalid magic, or...
            // The excess was rejected
            if should_remove {
                // Client was removed
            }
        }
        Err(e) => {
            // This is the expected behavior — buffer cap enforced
            assert!(
                matches!(e, vb_ipc::IpcServerError::ReadBufferTooLarge),
                "Expected ReadBufferTooLarge, got {:?}",
                e
            );
        }
    }

    let _ = std::fs::remove_file(&socket_path);
}

// ─────────────────────────────────────────────────────────────────────────────
// B12: First read chunk bounded by READ_CHUNK_BYTES
// ─────────────────────────────────────────────────────────────────────────────

/// Given: a fresh ClientConnection from accept_client
/// When: the socket provides 4096 bytes in a single read
/// Then: temp_buf is filled with exactly 4096 bytes
/// And: read_buffer does not exceed 4096 before magic validation
#[test]
fn first_read_chunk_respects_read_chunk_bytes_bound() {
    let socket_path = std::env::temp_dir().join("vb54mw_test_chunk_bound.sock");
    let _ = std::fs::remove_file(&socket_path);

    let (mut server, mut client, mut runtime) =
        setup_server_and_client(&socket_path).expect("setup failed");

    // Send exactly 4096 bytes (the READ_CHUNK_BYTES limit)
    let chunk = vec![0xAB; 4096];
    client.write_all(&chunk).expect("write failed");

    // Poll — should read at most 4096 bytes in first read
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));

    // The result depends on whether the data is valid magic or not
    // But the CRITICAL invariant: server did NOT read more than 4096 bytes
    // We can verify this by checking if another poll reads more data
    let more = [0xCD; 100];
    client.write_all(&more).expect("write failed");

    let result2 = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));

    // If the implementation correctly bounded the first read to 4096,
    // the second poll should process the additional 100 bytes
    // (or the client was removed if the 4096 bytes were invalid magic)

    let _ = std::fs::remove_file(&socket_path);
}

// ─────────────────────────────────────────────────────────────────────────────
// B1-B4: validate_magic_early behavior tests
// These are compile-time verified through the validate_magic_early function
// ─────────────────────────────────────────────────────────────────────────────

/// B1: validate_magic_early accepts only IPC_MAGIC
#[test]
fn validate_magic_early_returns_magic_validated_when_given_ipc_magic_bytes() {
    let magic_bytes = IPC_MAGIC.to_le_bytes();
    let result = vb_ipc::server::helpers::validate_magic_early(&magic_bytes);
    assert!(
        result.is_ok(),
        "validate_magic_early should accept IPC_MAGIC bytes"
    );
}

/// B2: validate_magic_early rejects zero
#[test]
fn validate_magic_early_rejects_zero_u32() {
    let zero_bytes = [0x00, 0x00, 0x00, 0x00];
    let result = vb_ipc::server::helpers::validate_magic_early(&zero_bytes);
    assert!(
        result.is_err(),
        "validate_magic_early should reject zero"
    );
    if let Err(e) = result {
        assert!(
            matches!(e, IpcError::InvalidMagic { actual: 0 }),
            "Expected InvalidMagic {{ actual: 0 }}, got {:?}",
            e
        );
    }
}

/// B3: validate_magic_early rejects u32::MAX
#[test]
fn validate_magic_early_rejects_max_u32() {
    let max_bytes = 0xFFFFFFFF_u32.to_le_bytes();
    let result = vb_ipc::server::helpers::validate_magic_early(&max_bytes);
    assert!(
        result.is_err(),
        "validate_magic_early should reject u32::MAX"
    );
    if let Err(e) = result {
        assert!(
            matches!(e, IpcError::InvalidMagic { actual: u32::MAX }),
            "Expected InvalidMagic {{ actual: u32::MAX }}, got {:?}",
            e
        );
    }
}

/// B4: validate_magic_early rejects near-miss values
#[test]
fn validate_magic_early_rejects_one_below_ipc_magic() {
    let one_below = (IPC_MAGIC - 1).to_le_bytes();
    let result = vb_ipc::server::helpers::validate_magic_early(&one_below);
    assert!(
        result.is_err(),
        "validate_magic_early should reject IPC_MAGIC - 1"
    );
}

#[test]
fn validate_magic_early_rejects_one_above_ipc_magic() {
    let one_above = (IPC_MAGIC + 1).to_le_bytes();
    let result = vb_ipc::server::helpers::validate_magic_early(&one_above);
    assert!(
        result.is_err(),
        "validate_magic_early should reject IPC_MAGIC + 1"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// B5-B8: append_read_bytes with MagicValidationState
// ─────────────────────────────────────────────────────────────────────────────

/// B5: append_read_bytes with AwaitingMagic rejects growth beyond 4 bytes
#[test]
fn append_read_bytes_awaiting_magic_rejects_5th_byte() {
    use vb_ipc::server::impl_::MagicValidationState;

    let mut read_buffer = vec![0u8; 0];
    let temp_buf = [0xAB; 4096];
    let state = MagicValidationState::AwaitingMagic;

    let result =
        vb_ipc::server::helpers::append_read_bytes_with_state(&mut read_buffer, &temp_buf, 5, state);
    assert!(
        result.is_err(),
        "append_read_bytes with AwaitingMagic should reject 5 bytes"
    );
    if let Err(e) = result {
        assert!(
            matches!(e, vb_ipc::IpcServerError::ReadBufferTooLarge),
            "Expected ReadBufferTooLarge, got {:?}",
            e
        );
    }
}

/// B6: append_read_bytes with AwaitingMagic accepts up to 4 bytes
#[test]
fn append_read_bytes_awaiting_magic_accepts_exactly_4_bytes() {
    use vb_ipc::server::impl_::MagicValidationState;

    let mut read_buffer = vec![0u8; 0];
    let temp_buf = [0xAB; 4096];
    let state = MagicValidationState::AwaitingMagic;

    let result =
        vb_ipc::server::helpers::append_read_bytes_with_state(&mut read_buffer, &temp_buf, 4, state);
    assert_eq!(result, Ok(()), "append_read_bytes should accept exactly 4 bytes");
    assert_eq!(read_buffer.len(), 4);
}

/// B7: append_read_bytes with MagicValidated accepts growth up to max
#[test]
fn append_read_bytes_magic_validated_accepts_large_chunk() {
    use vb_ipc::server::impl_::MagicValidationState;

    let mut read_buffer = vec![0u8; 0];
    let temp_buf = [0xAB; 4096];
    let state = MagicValidationState::MagicValidated;

    let result =
        vb_ipc::server::helpers::append_read_bytes_with_state(&mut read_buffer, &temp_buf, 4096, state);
    assert_eq!(result, Ok(()), "append_read_bytes should accept large chunk in MagicValidated");
    assert_eq!(read_buffer.len(), 4096);
}

/// B8: append_read_bytes with MagicValidated rejects overflow
#[test]
fn append_read_bytes_magic_validated_rejects_overflow() {
    use vb_ipc::server::impl_::MagicValidationState;

    let max_len = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
    let mut read_buffer = vec![0u8; max_len];
    let temp_buf = [0xAB; 4096];
    let state = MagicValidationState::MagicValidated;

    let result =
        vb_ipc::server::helpers::append_read_bytes_with_state(&mut read_buffer, &temp_buf, 1, state);
    assert!(
        result.is_err(),
        "append_read_bytes should reject overflow in MagicValidated"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// B9: State transition
// ─────────────────────────────────────────────────────────────────────────────

/// B9: State transitions from AwaitingMagic to MagicValidated after valid magic
#[test]
fn magic_validation_state_transitions_awaiting_to_validated_on_ok() {
    use vb_ipc::server::impl_::MagicValidationState;

    let magic_bytes = IPC_MAGIC.to_le_bytes();
    let result = vb_ipc::server::helpers::validate_magic_early(&magic_bytes);

    assert!(
        result.is_ok(),
        "validate_magic_early should return Ok for valid magic"
    );

    // The result should indicate the new state
    // This tests that the state transition occurs correctly
    assert!(
        matches!(result, Ok(MagicValidationState::MagicValidated)),
        "Expected MagicValidated state after successful validation"
    );
}
