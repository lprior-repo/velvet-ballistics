#![forbid(unsafe_code)]
//! Postcard newtype compatibility tests for `vb-dybj`.
//!
//! Freezes golden Postcard byte fixtures for `RunId`, `WorkflowDigest`, and
//! `RecordKind` surfaces so that accidental wire-format changes are caught
//! before they land.  Also asserts that malformed inputs (trailing bytes,
//! missing bytes) are rejected with typed errors.
//!
//! # Migration
//!
//! If a golden-byte constant must change, the commit message **must** include
//! the tag `vb-dybj-golden-fixture-migration` and the changelog must document
//! (a) which surface changed, (b) the before/after bytes, and (c) the
//! forward-compatibility plan.  Editing any fixture constant without that
//! documentation is a breaking change that will cause these tests to fail.

// ---------------------------------------------------------------------------
// Frozen golden-byte fixtures
// ---------------------------------------------------------------------------

/// RunId ZERO golden Postcard bytes.
///
/// Postcard encodes `u64` (and therefore `RunId`) as a varint.
/// `0_u64` → 1 byte: `0x00`.
///
/// Migration required name: `vb-dybj-run-id-zero-2026`
#[rustfmt::skip]
const RUN_ID_ZERO_POSTCARD_BYTES: &[u8] = &[
    0x00,
];

/// RunId MAX (u64::MAX) golden Postcard bytes.
///
/// Postcard varint for `u64::MAX` (0xFFFF_FFFF_FFFF_FFFF):
/// 9 × 0xFF continuation bytes + 1 × 0x01 terminator = 10 bytes.
///
/// Migration required name: `vb-dybj-run-id-max-2026`
#[rustfmt::skip]
const RUN_ID_MAX_POSTCARD_BYTES: &[u8] = &[
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
];

/// WorkflowDigest all-zero golden Postcard bytes.
///
/// Postcard serialises `[u8; 32]` as 32 raw bytes (no length prefix in the
/// default serde-via-slice path for fixed-size arrays).
///
/// Migration required name: `vb-dybj-workflow-digest-zero-2026`
#[rustfmt::skip]
const WORKFLOW_DIGEST_ZERO_POSTCARD_BYTES: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// WorkflowDigest nontrivial-pattern golden Postcard bytes.
///
/// Uses bytes `[0x00, 0x01, 0x02, ..., 0x1F]` (ascending 0..32).
///
/// Migration required name: `vb-dybj-workflow-digest-pattern-2026`
#[rustfmt::skip]
const WORKFLOW_DIGEST_PATTERN_POSTCARD_BYTES: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
];

/// RecordKind::RunHeader Postcard-enum golden bytes.
///
/// Serde's default enum derive serialises variants by declaration-order
/// index (zero-based), not by the explicit `#[repr(u16)]` discriminant.
/// RunHeader is the **3rd** declared variant → index 2.
/// Postcard varint for 2 is `[0x02]`.
///
/// **This surface differs from `RecordKind::id()` (envelope u16 LE).**
///
/// Migration required name: `vb-dybj-record-kind-run-header-2026`
#[rustfmt::skip]
const RECORD_KIND_RUN_HEADER_POSTCARD_BYTES: &[u8] = &[
    0x02,
];

/// RecordKind::RunAccepted Postcard-enum golden bytes.
///
/// RunAccepted is the **4th** declared variant → index 3.
/// Postcard varint for 3 is `[0x03]`.
///
/// **This surface differs from `RecordKind::id()` (envelope u16 LE).**
///
/// Migration required name: `vb-dybj-record-kind-run-accepted-2026`
#[rustfmt::skip]
const RECORD_KIND_RUN_ACCEPTED_POSTCARD_BYTES: &[u8] = &[
    0x03,
];

/// Canonical migration tag referenced by every golden-fixture constant.
pub const MIGRATION_REQUIRED_TAG: &str = "vb-dybj-golden-fixture-migration";

// =========================================================================
// Sub-module: run_id
// =========================================================================
mod run_id {
    use vb_core::RunId;

    /// Serialise `T` with postcard.  The test fails on serialisation errors.
    fn serialise<T: serde::Serialize>(value: &T) -> Vec<u8> {
        postcard::to_allocvec(value)
            .ok()
            .unwrap_or_else(|| unreachable!("serialise failed"))
    }

    fn deserialise<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
        postcard::from_bytes::<T>(bytes)
            .ok()
            .unwrap_or_else(|| unreachable!("deserialise failed"))
    }

    #[test]
    fn run_id_new_get_roundtrips_for_selected_u64_values() {
        let cases: &[(u64, &str)] = &[
            (0, "zero"),
            (1, "one"),
            (u64::MAX, "max"),
            (0xDEAD_BEEF_CAFE_BABE, "mid-range"),
        ];
        for (v, label) in cases {
            let id = RunId::new(*v);
            assert_eq!(id.get(), *v, "RunId::new({v}).get() [{label}]");
        }
    }

    #[test]
    fn run_id_new_get_roundtrips_for_edge_value_zero() {
        let id = RunId::new(0);
        assert_eq!(id.get(), 0);
    }

    #[test]
    fn run_id_new_get_roundtrips_for_edge_value_max_u64() {
        let id = RunId::new(u64::MAX);
        assert_eq!(id.get(), u64::MAX);
    }

    #[test]
    fn run_id_zero_constant_equals_run_id_new_zero() {
        assert_eq!(RunId::ZERO, RunId::new(0));
    }

    #[test]
    fn run_id_zero_postcard_bytes_match_run_id_new_zero_bytes() {
        let bz = serialise(&RunId::ZERO);
        let bn = serialise(&RunId::new(0));
        assert_eq!(
            bz, bn,
            "Postcard bytes for RunId::ZERO and RunId::new(0) must be identical"
        );
    }

    #[test]
    fn run_id_zero_postcard_bytes_equal_golden_fixture() {
        let bytes = serialise(&RunId::ZERO);
        assert_eq!(
            &bytes,
            super::RUN_ID_ZERO_POSTCARD_BYTES,
            "RunId::ZERO Postcard bytes must match golden fixture; \
             if changed, need {MIG}",
            MIG = super::MIGRATION_REQUIRED_TAG
        );
    }

    #[test]
    fn run_id_max_postcard_bytes_equal_golden_fixture() {
        let bytes = serialise(&RunId::new(u64::MAX));
        assert_eq!(
            &bytes,
            super::RUN_ID_MAX_POSTCARD_BYTES,
            "RunId::MAX Postcard bytes must match golden fixture; \
             if changed, need {MIG}",
            MIG = super::MIGRATION_REQUIRED_TAG
        );
    }

    #[test]
    fn run_id_decode_from_golden_fixture_zero_yields_run_id_zero() {
        let decoded: RunId = deserialise(super::RUN_ID_ZERO_POSTCARD_BYTES);
        assert_eq!(decoded, RunId::ZERO);
    }

    #[test]
    fn run_id_decode_from_golden_fixture_max_yields_run_id_max() {
        let decoded: RunId = deserialise(super::RUN_ID_MAX_POSTCARD_BYTES);
        assert_eq!(decoded, RunId::new(u64::MAX));
    }

    proptest::proptest! {
        #[test]
        fn run_id_postcard_roundtrip_holds_for_any_u64(v: u64) {
            let id = RunId::new(v);
            let bytes = postcard::to_allocvec(&id)
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("serialise: {e:?}")))?;
            let roundtripped: RunId = postcard::from_bytes(&bytes)
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("deserialise: {e:?}")))?;
            proptest::prop_assert_eq!(roundtripped, id);
        }
    }
}

// =========================================================================
// Sub-module: workflow_digest
// =========================================================================
mod workflow_digest {
    use vb_core::WorkflowDigest;

    fn serialise<T: serde::Serialize>(value: &T) -> Vec<u8> {
        postcard::to_allocvec(value)
            .ok()
            .unwrap_or_else(|| unreachable!("serialise failed"))
    }

    fn deserialise<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
        postcard::from_bytes::<T>(bytes)
            .ok()
            .unwrap_or_else(|| unreachable!("deserialise failed"))
    }

    fn nontrivial_bytes() -> [u8; 32] {
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            if let Ok(val) = u8::try_from(i) {
                *b = val;
            }
        }
        bytes
    }

    #[test]
    fn workflow_digest_from_bytes_as_bytes_roundtrip_for_zero_array() {
        let input = [0u8; 32];
        let digest = WorkflowDigest::from_bytes(input);
        assert_eq!(digest.as_bytes(), input);
    }

    #[test]
    fn workflow_digest_from_bytes_as_bytes_roundtrip_for_nontrivial_pattern() {
        let input = nontrivial_bytes();
        let digest = WorkflowDigest::from_bytes(input);
        assert_eq!(digest.as_bytes(), input);
    }

    #[test]
    fn workflow_digest_zero_postcard_bytes_equal_golden_fixture() {
        let digest = WorkflowDigest::from_bytes([0u8; 32]);
        let bytes = serialise(&digest);
        assert_eq!(
            &bytes,
            super::WORKFLOW_DIGEST_ZERO_POSTCARD_BYTES,
            "WorkflowDigest zero Postcard bytes must match golden fixture; \
             if changed, need {MIG}",
            MIG = super::MIGRATION_REQUIRED_TAG
        );
    }

    #[test]
    fn workflow_digest_nontrivial_postcard_bytes_equal_golden_fixture() {
        let digest = WorkflowDigest::from_bytes(nontrivial_bytes());
        let bytes = serialise(&digest);
        assert_eq!(
            &bytes,
            super::WORKFLOW_DIGEST_PATTERN_POSTCARD_BYTES,
            "WorkflowDigest pattern Postcard bytes must match golden fixture; \
             if changed, need {MIG}",
            MIG = super::MIGRATION_REQUIRED_TAG
        );
    }

    #[test]
    fn workflow_digest_decode_from_golden_fixture_yields_original() {
        let decoded: WorkflowDigest = deserialise(super::WORKFLOW_DIGEST_PATTERN_POSTCARD_BYTES);
        assert_eq!(decoded, WorkflowDigest::from_bytes(nontrivial_bytes()));
    }

    proptest::proptest! {
        #[test]
        fn workflow_digest_from_bytes_as_bytes_roundtrip_for_any_32_bytes(b: [u8; 32]) {
            let digest = WorkflowDigest::from_bytes(b);
            proptest::prop_assert_eq!(digest.as_bytes(), b);
        }

        #[test]
        fn workflow_digest_postcard_roundtrip_holds_for_any_32_bytes(b: [u8; 32]) {
            let digest = WorkflowDigest::from_bytes(b);
            let serialised = postcard::to_allocvec(&digest)
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("serialise: {e:?}")))?;
            let roundtripped: WorkflowDigest = postcard::from_bytes(&serialised)
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("deserialise: {e:?}")))?;
            proptest::prop_assert_eq!(roundtripped.as_bytes(), b);
        }
    }
}

// =========================================================================
// Sub-module: record_kind
// =========================================================================
mod record_kind {
    use vb_storage::records::RecordKind;

    fn serialise<T: serde::Serialize>(value: &T) -> Vec<u8> {
        postcard::to_allocvec(value)
            .ok()
            .unwrap_or_else(|| unreachable!("serialise failed"))
    }

    #[test]
    fn record_kind_run_header_envelope_id_u16_le_equals_3() {
        let id = RecordKind::RunHeader.id();
        assert_eq!(id, 3);
        assert_eq!(id.to_le_bytes(), [0x03, 0x00]);
    }

    #[test]
    fn record_kind_run_accepted_envelope_id_u16_le_equals_10() {
        let id = RecordKind::RunAccepted.id();
        assert_eq!(id, 10);
        assert_eq!(id.to_le_bytes(), [0x0A, 0x00]);
    }

    #[test]
    fn record_kind_run_header_postcard_enum_bytes_equal_golden_fixture() {
        let bytes = serialise(&RecordKind::RunHeader);
        assert_eq!(
            &bytes,
            super::RECORD_KIND_RUN_HEADER_POSTCARD_BYTES,
            "Postcard enum bytes for RunHeader must match golden fixture; \
             if changed, need {MIG}",
            MIG = super::MIGRATION_REQUIRED_TAG
        );
    }

    #[test]
    fn record_kind_run_accepted_postcard_enum_bytes_equal_golden_fixture() {
        let bytes = serialise(&RecordKind::RunAccepted);
        assert_eq!(
            &bytes,
            super::RECORD_KIND_RUN_ACCEPTED_POSTCARD_BYTES,
            "Postcard enum bytes for RunAccepted must match golden fixture; \
             if changed, need {MIG}",
            MIG = super::MIGRATION_REQUIRED_TAG
        );
    }

    #[test]
    fn record_kind_postcard_enum_bytes_differ_from_envelope_id_u16_le_run_header() {
        let postcard_bytes = serialise(&RecordKind::RunHeader);
        let envelope_bytes = RecordKind::RunHeader.id().to_le_bytes().to_vec();
        assert_ne!(
            postcard_bytes, envelope_bytes,
            "Postcard enum bytes for RunHeader (variant index varint) \
             must differ from envelope_id_u16_le bytes (discriminant u16 LE)"
        );
    }

    #[test]
    fn record_kind_postcard_enum_bytes_differ_from_envelope_id_u16_le_run_accepted() {
        let postcard_bytes = serialise(&RecordKind::RunAccepted);
        let envelope_bytes = RecordKind::RunAccepted.id().to_le_bytes().to_vec();
        assert_ne!(
            postcard_bytes, envelope_bytes,
            "Postcard enum bytes for RunAccepted (variant index varint) \
             must differ from envelope_id_u16_le bytes (discriminant u16 LE)"
        );
    }
}

// =========================================================================
// Sub-module: trailing_bytes
// =========================================================================
mod trailing_bytes {
    use vb_core::{RunId, WorkflowDigest};

    /// Exact-value decode: deserialise `T` and reject any unconsumed trailing bytes.
    /// Returns `Ok(value)` on success, `Err(message)` if trailing data exists.
    fn exact_decode_rejecting_trailing<T: serde::de::DeserializeOwned>(
        bytes: &[u8],
    ) -> Result<T, String> {
        let (value, remaining) =
            postcard::take_from_bytes::<T>(bytes).map_err(|e| format!("decode: {e:?}"))?;
        if remaining.is_empty() {
            Ok(value)
        } else {
            Err(format!("trailing {} byte(s) after decode", remaining.len()))
        }
    }

    fn serialise<T: serde::Serialize>(value: &T) -> Vec<u8> {
        postcard::to_allocvec(value)
            .ok()
            .unwrap_or_else(|| unreachable!("serialise failed"))
    }

    fn concat(base: &[u8], suffix: &[u8]) -> Vec<u8> {
        let cap = base.len().saturating_add(suffix.len());
        let mut buf = Vec::with_capacity(cap);
        buf.extend_from_slice(base);
        buf.extend_from_slice(suffix);
        buf
    }

    #[test]
    fn trailing_bytes_run_id_rejected_with_extra_byte() {
        let base = serialise(&RunId::ZERO);
        let bad = concat(&base, &[0xFF]);
        let result = exact_decode_rejecting_trailing::<RunId>(&bad);
        assert!(
            result.is_err(),
            "single extra byte must be rejected; got {:?}",
            result
        );
    }

    #[test]
    fn trailing_bytes_run_id_rejected_with_multiple_extra_bytes() {
        let base = serialise(&RunId::ZERO);
        let bad = concat(&base, &[0xAA; 10]);
        let result = exact_decode_rejecting_trailing::<RunId>(&bad);
        assert!(
            result.is_err(),
            "multiple extra bytes must be rejected; got {:?}",
            result
        );
    }

    #[test]
    fn trailing_bytes_workflow_digest_rejected_with_extra_byte() {
        let base = serialise(&WorkflowDigest::from_bytes([0u8; 32]));
        let bad = concat(&base, &[0xFF]);
        let result = exact_decode_rejecting_trailing::<WorkflowDigest>(&bad);
        assert!(
            result.is_err(),
            "single extra byte must be rejected; got {:?}",
            result
        );
    }

    #[test]
    fn trailing_bytes_workflow_digest_rejected_with_multiple_extra_bytes() {
        let base = serialise(&WorkflowDigest::from_bytes([0u8; 32]));
        let bad = concat(&base, &[0xAA; 10]);
        let result = exact_decode_rejecting_trailing::<WorkflowDigest>(&bad);
        assert!(
            result.is_err(),
            "multiple extra bytes must be rejected; got {:?}",
            result
        );
    }

    proptest::proptest! {
        #[test]
        fn trailing_bytes_rejected_for_any_suffix_on_run_id(
            v: u64,
            suffix in proptest::collection::vec(proptest::prelude::any::<u8>(), 1..=64),
        ) {
            let base = postcard::to_allocvec(&RunId::new(v))
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("serialise: {e:?}")))?;
            let bad = concat(&base, &suffix);
            let result = exact_decode_rejecting_trailing::<RunId>(&bad);
            proptest::prop_assert!(
                result.is_err(),
                "RunId with trailing {} byte(s) must be rejected", suffix.len()
            );
        }

        #[test]
        fn trailing_bytes_rejected_for_any_suffix_on_workflow_digest(
            b: [u8; 32],
            suffix in proptest::collection::vec(proptest::prelude::any::<u8>(), 1..=64),
        ) {
            let base = postcard::to_allocvec(&WorkflowDigest::from_bytes(b))
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("serialise: {e:?}")))?;
            let bad = concat(&base, &suffix);
            let result = exact_decode_rejecting_trailing::<WorkflowDigest>(&bad);
            proptest::prop_assert!(
                result.is_err(),
                "WorkflowDigest with trailing {} byte(s) must be rejected", suffix.len()
            );
        }
    }
}

// =========================================================================
// Sub-module: missing_bytes
// =========================================================================
mod missing_bytes {
    use vb_storage::codec::decode_record_header;
    use vb_storage::constants::{
        MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES,
    };
    use vb_storage::error::JournalError;

    #[test]
    fn decode_record_header_returns_unexpected_eof_for_zero_bytes() {
        let result =
            decode_record_header(&[], MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "zero-length input must yield UnexpectedEof, got {:?}",
            result
        );
    }

    #[test]
    fn decode_record_header_returns_unexpected_eof_for_one_byte() {
        let result = decode_record_header(
            &[0x00],
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "1-byte input must yield UnexpectedEof, got {:?}",
            result
        );
    }

    #[test]
    fn decode_record_header_returns_unexpected_eof_for_header_minus_one_bytes() {
        let input = vec![0x00u8; RECORD_HEADER_BYTES.saturating_sub(1)];
        let result =
            decode_record_header(&input, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "{}-byte input (one short) must yield UnexpectedEof, got {:?}",
            input.len(),
            result
        );
    }

    #[test]
    fn decode_record_header_does_not_return_unexpected_eof_for_exact_header_length() {
        let input = vec![0x00u8; RECORD_HEADER_BYTES];
        let result =
            decode_record_header(&input, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            !matches!(result, Err(JournalError::UnexpectedEof)),
            "Exactly {} bytes must NOT yield UnexpectedEof",
            RECORD_HEADER_BYTES
        );
    }

    #[test]
    fn decode_record_returns_postcard_decode_failed_for_corrupted_payload() {
        // Strategy: build a syntactically valid header that declares a payload,
        // then attach random bytes so that Postcard decode fails on the payload.
        use vb_storage::codec::encode_record_header;
        use vb_storage::records::RecordKind;

        let garbage_payload = b"not valid postcard";
        let header = match encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            1,
            garbage_payload,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) {
            Ok(h) => h,
            Err(e) => {
                unreachable!("header encode failed: {e:?}")
            }
        };

        let payload_len = header.len().saturating_add(garbage_payload.len());
        let mut full_record = Vec::with_capacity(payload_len);
        full_record.extend_from_slice(&header);
        full_record.extend_from_slice(garbage_payload);

        let result = vb_storage::codec::decode_record::<vb_storage::records::RunHeaderRecord>(
            &full_record,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PostcardDecodeFailed)),
            "garbage payload must yield PostcardDecodeFailed, got {:?}",
            result
        );
    }

    proptest::proptest! {
        #[test]
        fn decode_record_header_returns_unexpected_eof_for_any_short_input(
            short_bytes in proptest::collection::vec(
                proptest::prelude::any::<u8>(),
                0..RECORD_HEADER_BYTES,
            )
        ) {
            let result =
                decode_record_header(&short_bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
            proptest::prop_assert!(
                matches!(result, Err(JournalError::UnexpectedEof)),
                "{} bytes (< {}) must yield UnexpectedEof, got {:?}",
                short_bytes.len(), RECORD_HEADER_BYTES, result
            );
        }
    }
}

// =========================================================================
// Sub-module: migration_required
// =========================================================================
mod migration_required {
    use vb_core::RunId;

    #[test]
    fn migration_required_run_id_zero_byte_change_without_migration_name_fails() {
        let bytes = match postcard::to_allocvec(&RunId::ZERO) {
            Ok(bytes) => bytes,
            Err(e) => {
                unreachable!("serialise RunId::ZERO: {e:?}")
            }
        };
        assert_eq!(
            &bytes,
            super::RUN_ID_ZERO_POSTCARD_BYTES,
            "RunId::ZERO bytes changed without {MIG} migration; produced {bytes:02X?}",
            MIG = super::MIGRATION_REQUIRED_TAG,
        );
    }

    #[test]
    fn migration_required_workflow_digest_byte_change_without_migration_name_fails() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let bytes = match postcard::to_allocvec(&digest) {
            Ok(bytes) => bytes,
            Err(e) => {
                unreachable!("serialise WorkflowDigest: {e:?}")
            }
        };
        assert_eq!(
            &bytes,
            super::WORKFLOW_DIGEST_ZERO_POSTCARD_BYTES,
            "WorkflowDigest zero bytes changed without {MIG} migration; produced {bytes:02X?}",
            MIG = super::MIGRATION_REQUIRED_TAG,
        );
    }

    #[test]
    fn migration_required_record_kind_byte_change_without_migration_name_fails() {
        use vb_storage::records::RecordKind;
        let bytes = match postcard::to_allocvec(&RecordKind::RunHeader) {
            Ok(bytes) => bytes,
            Err(e) => {
                unreachable!("serialise RecordKind::RunHeader: {e:?}")
            }
        };
        assert_eq!(
            &bytes,
            super::RECORD_KIND_RUN_HEADER_POSTCARD_BYTES,
            "RecordKind::RunHeader bytes changed without {MIG} migration; produced {bytes:02X?}",
            MIG = super::MIGRATION_REQUIRED_TAG,
        );
    }

    #[test]
    fn migration_required_tag_is_nonempty() {
        assert!(!super::MIGRATION_REQUIRED_TAG.is_empty());
    }
}
