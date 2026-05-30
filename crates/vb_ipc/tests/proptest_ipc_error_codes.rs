//! Property test: Every IpcError::diagnostic_code() returns the correct
//! documented constant (all 14 variants).
//!
//! PO-007 / PS-007: Error code stability — diagnostic_code correct for all IpcError variants.
//!
//! Each variant's expected code is the const defined in error.rs (0x3001–0x300E).

use vb_core::DiagnosticCode;
use vb_ipc::IpcError;

#[test]
fn full_returns_correct_code() {
    let err = IpcError::Full;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3001),
        "FULL_CODE = 0x3001"
    );
}

#[test]
fn disconnected_returns_correct_code() {
    let err = IpcError::Disconnected;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3002),
        "DISCONNECTED_CODE = 0x3002"
    );
}

#[test]
fn payload_too_large_returns_correct_code() {
    let err = IpcError::PayloadTooLarge {
        actual: 2000,
        limit: 1000,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3003),
        "PAYLOAD_TOO_LARGE_CODE = 0x3003"
    );
}

#[test]
fn invalid_magic_returns_correct_code() {
    let err = IpcError::InvalidMagic { actual: 0xDEAD };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3004),
        "INVALID_MAGIC_CODE = 0x3004"
    );
}

#[test]
fn unsupported_version_returns_correct_code() {
    let err = IpcError::UnsupportedVersion { actual: 99 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3005),
        "UNSUPPORTED_VERSION_CODE = 0x3005"
    );
}

#[test]
fn unknown_command_returns_correct_code() {
    let err = IpcError::UnknownCommand(0xFF);
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3006),
        "UNKNOWN_COMMAND_CODE = 0x3006"
    );
}

#[test]
fn reserved_non_zero_returns_correct_code() {
    let err = IpcError::ReservedNonZero { actual: 1 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3007),
        "RESERVED_NON_ZERO_CODE = 0x3007"
    );
}

#[test]
fn payload_length_mismatch_returns_correct_code() {
    let err = IpcError::PayloadLengthMismatch {
        header: 100,
        actual: 80,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3008),
        "PAYLOAD_LENGTH_MISMATCH_CODE = 0x3008"
    );
}

#[test]
fn header_encode_failed_returns_correct_code() {
    let err = IpcError::HeaderEncodeFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x3009),
        "HEADER_ENCODE_FAILED_CODE = 0x3009"
    );
}

#[test]
fn header_decode_failed_returns_correct_code() {
    let err = IpcError::HeaderDecodeFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x300A),
        "HEADER_DECODE_FAILED_CODE = 0x300A"
    );
}

#[test]
fn payload_length_out_of_range_returns_correct_code() {
    let err = IpcError::PayloadLengthOutOfRange { actual: u32::MAX };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x300B),
        "PAYLOAD_LENGTH_OUT_OF_RANGE_CODE = 0x300B"
    );
}

#[test]
fn payload_encode_failed_returns_correct_code() {
    let err = IpcError::PayloadEncodeFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x300C),
        "PAYLOAD_ENCODE_FAILED_CODE = 0x300C"
    );
}

#[test]
fn payload_decode_failed_returns_correct_code() {
    let err = IpcError::PayloadDecodeFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x300D),
        "PAYLOAD_DECODE_FAILED_CODE = 0x300D"
    );
}

#[test]
fn response_decode_failed_returns_correct_code() {
    let err = IpcError::ResponseDecodeFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x300E),
        "RESPONSE_DECODE_FAILED_CODE = 0x300E"
    );
}

#[test]
fn all_14_ipc_error_variants_nonzero() {
    let variants: &[IpcError] = &[
        IpcError::Full,
        IpcError::Disconnected,
        IpcError::PayloadTooLarge {
            actual: 2000,
            limit: 1000,
        },
        IpcError::InvalidMagic { actual: 0xDEAD },
        IpcError::UnsupportedVersion { actual: 99 },
        IpcError::UnknownCommand(0xFF),
        IpcError::ReservedNonZero { actual: 1 },
        IpcError::PayloadLengthMismatch {
            header: 100,
            actual: 80,
        },
        IpcError::HeaderEncodeFailed,
        IpcError::HeaderDecodeFailed,
        IpcError::PayloadLengthOutOfRange { actual: u32::MAX },
        IpcError::PayloadEncodeFailed,
        IpcError::PayloadDecodeFailed,
        IpcError::ResponseDecodeFailed,
    ];

    assert_eq!(variants.len(), 14, "Expected 14 IpcError variants");

    // All codes are non-zero
    for err in variants {
        let code = err.diagnostic_code();
        assert_ne!(
            code.code(),
            0,
            "IpcError variant returned zero code"
        );
    }

    // Note: IpcError codes (0x3001-0x300E) are in the Runtime E30xx range
    // of CODE_REGISTRY, not the IPC E32xx range. This is an existing design
    // characteristic (IpcError codes were allocated in the 0x30xx space before
    // the IPC category was split into its own E32xx range).
    // We do not assert CODE_REGISTRY membership for IpcError codes;
    // the correct-code-constant tests above provide the primary verification.
}
