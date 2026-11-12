use super::{IpcError, IpcServerError};
use std::io;

fn io_error(kind: io::ErrorKind, msg: &str) -> io::Error {
    io::Error::new(kind, msg)
}

// ── Display message tests ──

#[test]
fn bind_failed_display_mentions_bind() {
    let err = IpcServerError::BindFailed {
        source: io_error(io::ErrorKind::AddrInUse, "addr in use"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("bind failed"),
        "expected 'bind failed' in '{msg}'"
    );
}

#[test]
fn poll_failed_display_mentions_poll() {
    let err = IpcServerError::PollFailed {
        source: io_error(io::ErrorKind::Interrupted, "interrupted"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("poll failed"),
        "expected 'poll failed' in '{msg}'"
    );
}

#[test]
fn accept_failed_display_mentions_accept() {
    let err = IpcServerError::AcceptFailed {
        source: io_error(io::ErrorKind::ConnectionRefused, "refused"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("accept failed"),
        "expected 'accept failed' in '{msg}'"
    );
}

#[test]
fn too_many_clients_display() {
    let err = IpcServerError::TooManyClients;
    let msg = err.to_string();
    assert!(
        msg.contains("too many clients"),
        "expected 'too many clients' in '{msg}'"
    );
}

#[test]
fn response_encode_failed_display() {
    let err = IpcServerError::ResponseEncodeFailed;
    let msg = err.to_string();
    assert!(
        msg.contains("response encode failed"),
        "expected 'response encode failed' in '{msg}'"
    );
}

#[test]
fn response_write_failed_display() {
    let err = IpcServerError::ResponseWriteFailed {
        source: io_error(io::ErrorKind::BrokenPipe, "pipe broke"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("response write failed"),
        "expected 'response write failed' in '{msg}'"
    );
}

#[test]
fn incomplete_frame_display() {
    let err = IpcServerError::IncompleteFrame;
    let msg = err.to_string();
    assert!(
        msg.contains("incomplete IPC frame"),
        "expected 'incomplete IPC frame' in '{msg}'"
    );
}

#[test]
fn read_buffer_too_large_display() {
    let err = IpcServerError::ReadBufferTooLarge;
    let msg = err.to_string();
    assert!(
        msg.contains("read buffer exceeded"),
        "expected 'read buffer exceeded' in '{msg}'"
    );
}

#[test]
fn frame_invalid_display_wraps_source() {
    let err = IpcServerError::FrameInvalid {
        source: IpcError::InvalidMagic { actual: 0xDEAD },
    };
    let msg = err.to_string();
    assert!(
        msg.contains("invalid IPC frame"),
        "expected 'invalid IPC frame' in '{msg}'"
    );
    assert!(
        msg.contains("magic"),
        "expected inner error detail in '{msg}'"
    );
}

// ── Runtime code mapping tests ──

#[test]
fn runtime_code_incomplete_frame_maps_to_frame_invalid() {
    let err = IpcServerError::IncompleteFrame;
    assert_eq!(err.runtime_code(), Some("IPC_FRAME_INVALID"));
}

#[test]
fn runtime_code_read_buffer_too_large_maps_to_payload_too_large() {
    let err = IpcServerError::ReadBufferTooLarge;
    assert_eq!(err.runtime_code(), Some("IPC_PAYLOAD_TOO_LARGE"));
}

#[test]
fn runtime_code_frame_invalid_delegates_to_source() {
    let err = IpcServerError::FrameInvalid {
        source: IpcError::InvalidMagic { actual: 0 },
    };
    assert_eq!(err.runtime_code(), Some("IPC_FRAME_INVALID"));
}

#[test]
fn runtime_code_frame_invalid_delegates_to_payload_too_large_source() {
    let err = IpcServerError::FrameInvalid {
        source: IpcError::PayloadTooLarge {
            actual: 99,
            limit: 10,
        },
    };
    assert_eq!(err.runtime_code(), Some("IPC_PAYLOAD_TOO_LARGE"));
}

#[test]
fn runtime_code_too_many_clients_maps_to_queue_full() {
    let err = IpcServerError::TooManyClients;
    assert_eq!(err.runtime_code(), Some("QUEUE_FULL"));
}

#[test]
fn runtime_code_bind_failed_is_none() {
    let err = IpcServerError::BindFailed {
        source: io_error(io::ErrorKind::AddrInUse, "addr"),
    };
    assert_eq!(err.runtime_code(), None);
}

#[test]
fn runtime_code_poll_failed_is_none() {
    let err = IpcServerError::PollFailed {
        source: io_error(io::ErrorKind::Interrupted, "intr"),
    };
    assert_eq!(err.runtime_code(), None);
}

#[test]
fn runtime_code_accept_failed_is_none() {
    let err = IpcServerError::AcceptFailed {
        source: io_error(io::ErrorKind::ConnectionRefused, "ref"),
    };
    assert_eq!(err.runtime_code(), None);
}

#[test]
fn runtime_code_response_encode_failed_is_none() {
    let err = IpcServerError::ResponseEncodeFailed;
    assert_eq!(err.runtime_code(), None);
}

#[test]
fn runtime_code_response_write_failed_is_none() {
    let err = IpcServerError::ResponseWriteFailed {
        source: io_error(io::ErrorKind::BrokenPipe, "pipe"),
    };
    assert_eq!(err.runtime_code(), None);
}

#[test]
fn runtime_code_frame_invalid_with_disconnected_source_is_none() {
    let err = IpcServerError::FrameInvalid {
        source: IpcError::Disconnected,
    };
    assert_eq!(err.runtime_code(), None);
}

// ── PartialEq tests ──

#[test]
fn partial_eq_bind_failed_same_error() {
    let a = IpcServerError::BindFailed {
        source: io_error(io::ErrorKind::AddrInUse, "addr"),
    };
    let b = IpcServerError::BindFailed {
        source: io_error(io::ErrorKind::AddrInUse, "addr"),
    };
    assert_eq!(a, b);
}

#[test]
fn partial_eq_bind_failed_different_message() {
    let a = IpcServerError::BindFailed {
        source: io_error(io::ErrorKind::AddrInUse, "addr1"),
    };
    let b = IpcServerError::BindFailed {
        source: io_error(io::ErrorKind::AddrInUse, "addr2"),
    };
    assert_ne!(a, b);
}

#[test]
fn partial_eq_poll_failed_same_error() {
    let a = IpcServerError::PollFailed {
        source: io_error(io::ErrorKind::Interrupted, "intr"),
    };
    let b = IpcServerError::PollFailed {
        source: io_error(io::ErrorKind::Interrupted, "intr"),
    };
    assert_eq!(a, b);
}

#[test]
fn partial_eq_poll_failed_different_message() {
    let a = IpcServerError::PollFailed {
        source: io_error(io::ErrorKind::Interrupted, "intr1"),
    };
    let b = IpcServerError::PollFailed {
        source: io_error(io::ErrorKind::Interrupted, "intr2"),
    };
    assert_ne!(a, b);
}

#[test]
fn partial_eq_accept_failed_same_error() {
    let a = IpcServerError::AcceptFailed {
        source: io_error(io::ErrorKind::ConnectionRefused, "ref"),
    };
    let b = IpcServerError::AcceptFailed {
        source: io_error(io::ErrorKind::ConnectionRefused, "ref"),
    };
    assert_eq!(a, b);
}

#[test]
fn partial_eq_accept_failed_different_message() {
    let a = IpcServerError::AcceptFailed {
        source: io_error(io::ErrorKind::ConnectionRefused, "ref1"),
    };
    let b = IpcServerError::AcceptFailed {
        source: io_error(io::ErrorKind::ConnectionRefused, "ref2"),
    };
    assert_ne!(a, b);
}

#[test]
fn partial_eq_response_write_failed_same_error() {
    let a = IpcServerError::ResponseWriteFailed {
        source: io_error(io::ErrorKind::BrokenPipe, "pipe"),
    };
    let b = IpcServerError::ResponseWriteFailed {
        source: io_error(io::ErrorKind::BrokenPipe, "pipe"),
    };
    assert_eq!(a, b);
}

#[test]
fn partial_eq_response_write_failed_different_message() {
    let a = IpcServerError::ResponseWriteFailed {
        source: io_error(io::ErrorKind::BrokenPipe, "pipe1"),
    };
    let b = IpcServerError::ResponseWriteFailed {
        source: io_error(io::ErrorKind::BrokenPipe, "pipe2"),
    };
    assert_ne!(a, b);
}

#[test]
fn partial_eq_too_many_clients_equal() {
    assert_eq!(
        IpcServerError::TooManyClients,
        IpcServerError::TooManyClients
    );
}

#[test]
fn partial_eq_response_encode_failed_equal() {
    assert_eq!(
        IpcServerError::ResponseEncodeFailed,
        IpcServerError::ResponseEncodeFailed
    );
}

#[test]
fn partial_eq_incomplete_frame_equal() {
    assert_eq!(
        IpcServerError::IncompleteFrame,
        IpcServerError::IncompleteFrame
    );
}

#[test]
fn partial_eq_read_buffer_too_large_equal() {
    assert_eq!(
        IpcServerError::ReadBufferTooLarge,
        IpcServerError::ReadBufferTooLarge
    );
}

#[test]
fn partial_eq_frame_invalid_same_source() {
    let a = IpcServerError::FrameInvalid {
        source: IpcError::InvalidMagic { actual: 0 },
    };
    let b = IpcServerError::FrameInvalid {
        source: IpcError::InvalidMagic { actual: 0 },
    };
    assert_eq!(a, b);
}

#[test]
fn partial_eq_different_variants_not_equal() {
    assert_ne!(
        IpcServerError::TooManyClients,
        IpcServerError::IncompleteFrame
    );
    assert_ne!(
        IpcServerError::BindFailed {
            source: io_error(io::ErrorKind::AddrInUse, "addr"),
        },
        IpcServerError::PollFailed {
            source: io_error(io::ErrorKind::Interrupted, "intr"),
        }
    );
}
