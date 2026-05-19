#![forbid(unsafe_code)]

use velvet_ballastics_workspace_tests::acceptance_catalog::{
    ExpectedDiagnostic, FailureFamily, FailureTaxonomyScenario, run_failure_taxonomy_scenario,
};

#[test]
fn corrupt_storage_record_returns_exact_storage_error() {
    let cases = [
        (
            "VB-82AH-STORAGE-WRONG-RUN",
            "JournalError::WrongRun",
            "0x4008",
        ),
        (
            "VB-82AH-STORAGE-SEQUENCE-GAP",
            "JournalError::SequenceGap",
            "0x4009",
        ),
        (
            "VB-82AH-STORAGE-SEQUENCE-OVERFLOW",
            "JournalError::SequenceOverflow",
            "0x400A",
        ),
        (
            "VB-82AH-STORAGE-BAD-MAGIC",
            "JournalError::BadMagic",
            "0x400B",
        ),
        (
            "VB-82AH-STORAGE-UNSUPPORTED-SCHEMA",
            "JournalError::UnsupportedSchemaVersion",
            "0x400C",
        ),
        (
            "VB-82AH-STORAGE-UNKNOWN-KIND",
            "JournalError::UnknownRecordKind",
            "0x400E",
        ),
        (
            "VB-82AH-STORAGE-HEADER-LENGTH",
            "JournalError::HeaderLengthMismatch",
            "0x4010",
        ),
        (
            "VB-82AH-STORAGE-PAYLOAD-TOO-LARGE",
            "JournalError::PayloadTooLarge",
            "0x4011",
        ),
        (
            "VB-82AH-STORAGE-HEADER-CHECKSUM",
            "JournalError::HeaderChecksumMismatch",
            "0x4012",
        ),
        (
            "VB-82AH-STORAGE-PAYLOAD-DIGEST",
            "JournalError::PayloadDigestMismatch",
            "0x4013",
        ),
        (
            "VB-82AH-STORAGE-UNEXPECTED-EOF",
            "JournalError::UnexpectedEof",
            "0x4014",
        ),
        (
            "VB-82AH-STORAGE-POSTCARD",
            "JournalError::PostcardDecodeFailed",
            "0x4015",
        ),
    ];

    for (id, typed_error, code) in cases {
        let scenario =
            FailureTaxonomyScenario::storage_corruption_fixture(id).with_expected_diagnostic(
                ExpectedDiagnostic::new(FailureFamily::StorageRecovery, typed_error, code, 5),
            );

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.journal_appended(), false);
    }
}

#[test]
fn replay_divergence_returns_exact_error_and_is_read_only() {
    let scenario = FailureTaxonomyScenario::replay_divergence_fixture("VB-82AH-REPLAY-DIVERGED")
        .with_expected_diagnostic(ExpectedDiagnostic::new(
            FailureFamily::Replay,
            "JournalError::PayloadDigestMismatch",
            "0x4013",
            8,
        ));

    let evidence = run_failure_taxonomy_scenario(&scenario);

    assert_eq!(
        evidence.typed_error(),
        "JournalError::PayloadDigestMismatch"
    );
    assert_eq!(evidence.diagnostic_code(), "0x4013");
    assert_eq!(evidence.cli_exit_code(), Some(8));
    assert_eq!(evidence.journal_digest_unchanged(), true);
    assert_eq!(evidence.journal_appended(), false);
}

#[test]
fn ipc_invalid_frames_map_to_all_required_e300x_codes() {
    let cases = [
        (
            "VB-82AH-IPC-FULL",
            "IpcError::Full",
            "E3001",
            Some("QUEUE_FULL"),
        ),
        (
            "VB-82AH-IPC-DISCONNECTED",
            "IpcError::Disconnected",
            "E3002",
            None,
        ),
        (
            "VB-82AH-IPC-OVERSIZE",
            "IpcError::PayloadTooLarge",
            "E3003",
            Some("IPC_PAYLOAD_TOO_LARGE"),
        ),
        (
            "VB-82AH-IPC-MAGIC",
            "IpcError::InvalidMagic",
            "E3004",
            Some("IPC_FRAME_INVALID"),
        ),
        (
            "VB-82AH-IPC-VERSION",
            "IpcError::UnsupportedVersion",
            "E3005",
            Some("IPC_FRAME_INVALID"),
        ),
        (
            "VB-82AH-IPC-COMMAND",
            "IpcError::UnknownCommand",
            "E3006",
            Some("IPC_FRAME_INVALID"),
        ),
        (
            "VB-82AH-IPC-RESERVED",
            "IpcError::ReservedNonZero",
            "E3007",
            Some("IPC_FRAME_INVALID"),
        ),
        (
            "VB-82AH-IPC-LENGTH",
            "IpcError::PayloadLengthMismatch",
            "E3008",
            Some("IPC_FRAME_INVALID"),
        ),
        (
            "VB-82AH-IPC-HEADER",
            "IpcError::HeaderDecodeFailed",
            "E300A",
            Some("IPC_FRAME_INVALID"),
        ),
        (
            "VB-82AH-IPC-RANGE",
            "IpcError::PayloadLengthOutOfRange",
            "E300B",
            Some("IPC_PAYLOAD_TOO_LARGE"),
        ),
        (
            "VB-82AH-IPC-PAYLOAD",
            "IpcError::PayloadDecodeFailed",
            "E300D",
            Some("IPC_FRAME_INVALID"),
        ),
        (
            "VB-82AH-IPC-RESPONSE",
            "IpcError::ResponseDecodeFailed",
            "E300E",
            Some("IPC_FRAME_INVALID"),
        ),
    ];

    for (id, typed_error, code, runtime_code) in cases {
        let scenario = FailureTaxonomyScenario::ipc_frame_fixture(id).with_expected_diagnostic(
            ExpectedDiagnostic::new(FailureFamily::Ipc, typed_error, code, 6),
        );

        let evidence = run_failure_taxonomy_scenario(&scenario);

        assert_eq!(evidence.typed_error(), typed_error);
        assert_eq!(evidence.diagnostic_code(), code);
        assert_eq!(evidence.runtime_code(), runtime_code);
    }
}
