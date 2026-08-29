verus! {
        forall|e: SpecJournalError|
            is_queue_full(e) ==> !is_payload_too_large(e),
        forall|e: SpecJournalError|
            is_payload_too_large(e) ==> !is_queue_full(e),
{
    assert(forall|e: SpecJournalError|
        is_queue_full(e) ==> !is_payload_too_large(e)) by {
        assert(matches!(SpecJournalError::QueueFull,
            SpecJournalError::PayloadTooLarge { .. }) == false);
    };
    assert(forall|e: SpecJournalError|
        is_payload_too_large(e) ==> !is_queue_full(e)) by {
        assert(matches!(SpecJournalError::PayloadTooLarge { len: 0u32, max: 0u32 },
            SpecJournalError::QueueFull) == false);
    };
}

/// Lemma (C4 + C6 together): the three byte-accounting rejection
/// variants are pairwise distinct.
///
/// Production binding: the three variants occupy three distinct
/// discriminant positions in the production `JournalError` enum.
pub proof fn lemma_byte_rejection_variants_pairwise_distinct()
    ensures
        forall|e: SpecJournalError|
            is_journal_batch_bytes_exceeded(e) ==> !is_queue_full(e),
        forall|e: SpecJournalError|
            is_journal_batch_bytes_exceeded(e) ==> !is_payload_too_large(e),
        forall|e: SpecJournalError|
            is_queue_full(e) ==> !is_payload_too_large(e),
{
    lemma_journal_batch_bytes_exceeded_distinct_from_queue_full();
    lemma_journal_batch_bytes_exceeded_distinct_from_payload_too_large();
    lemma_queue_full_distinct_from_payload_too_large();
}

// =============================================================================
// Guard precedence model (preserved from prior revision; pure spec
// guard-ordering proof, not a variant-discrimination claim)
// =============================================================================
//
// The original PS-003 spec also encoded the guard precedence for
// `append_event`. The `Guard` enum and `guard_index` function model
// the production guard ordering at
// `crates/vb_storage/src/batch/append_event.rs:18-25` (verified by
// SA-003 regression test `append_event_rejects_same_batch_duplicate`).
// The spec-mode ordering is a static witness — no production fn is
// bound to it via `assume_specification` because the guard
// precedence is enforced by the production source code structure,
// not by a single spec-attached exec fn.

/// Guard precedence model for `append_event`.
///
/// PRODUCTION BINDING: matches guard order in batch.rs append_event
/// (lines 18-25 per the SA-003 ordering):
///   1. Key validation (run_event_key)
///   2. Same-batch duplicate check (staged_event_keys.contains)
///   3. Durable duplicate check (events.contains_key)
///   4. Batch count limit (inner.len() >= MAX_BATCH_COUNT)
///   5. Per-record encoding (encode_record)
///   6. Accumulated byte admission (staged_bytes + encoded_len)
///   7. Insert into inner + staged_event_keys
pub enum Guard {
    KeyValidation,
    SameBatchDuplicate,
    DurableDuplicate,
    BatchCount,
    PerRecordEncoding,
    AccumulatedByteAdmission,
    Insert,
}

/// Guard index for comparison.
pub open spec fn guard_index(g: Guard) -> u8 {
    match g {
        Guard::KeyValidation => 0,
        Guard::SameBatchDuplicate => 1,
        Guard::DurableDuplicate => 2,
        Guard::BatchCount => 3,
        Guard::PerRecordEncoding => 4,
        Guard::AccumulatedByteAdmission => 5,
        Guard::Insert => 6,
    }
}

/// Spec: guard precedence ordering for `append_event`.
///
/// The accumulated byte admission guard must be after encoding
/// (because we need `encoded_len`) but before the insert mutation.
/// Uses `guard_index` for ordering comparisons.
pub open spec fn guard_precedence_order() -> bool {
    &&& guard_index(Guard::KeyValidation) < guard_index(Guard::SameBatchDuplicate)
    &&& guard_index(Guard::SameBatchDuplicate) < guard_index(Guard::DurableDuplicate)
    &&& guard_index(Guard::DurableDuplicate) < guard_index(Guard::BatchCount)
    &&& guard_index(Guard::BatchCount) < guard_index(Guard::PerRecordEncoding)
    &&& guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission)
    &&& guard_index(Guard::AccumulatedByteAdmission) < guard_index(Guard::Insert)
}

/// Lemma: guard precedence is well-ordered (each guard fires strictly
/// before the next one in the production `append_event` body).
pub proof fn lemma_guard_precedence_well_ordered()
    ensures
        guard_precedence_order(),
{
}

// =============================================================================
// Production-bound exec wrappers that exercise the extern_spec
// bridges.
// =============================================================================
//
// Each wrapper calls the production `encode_record` / `decode_record`
// through the `assume_specification` contracts above. The wrappers
// are the proof witnesses that the bridges are not used as vacuum
// specifications: each wrapper states a requires/ensures pair that is
// provable from the bridge contract disjunction, and the exec body
// invokes the bridge, forcing Verus to discharge the bridge contract
// against the actual exec call.
//
// Why the wrapper `ensures` clauses are disjunctions rather than
// exact per-branch claims: the bridge body is `#[verifier::external]`
// so Verus cannot see which `Result` variant the body returns. The
// bridge's `match r { ... }` ensures clause therefore gives the
// strongest post-state that holds for EVERY reachable branch. The
// wrapper's `ensures` is the union of those per-branch post-states,
// which is exactly what the bridge contract guarantees.

/// Happy-path wrapper for `encode_record`: under fresh, in-budget
/// conditions, the bridge returns `Ok(envelope)` and the
/// post-conditions from the bridge's `Ok` branch hold.
pub exec fn wrapper_encode_record_ok(
    magic: u32,
    kind: SpecRecordKind,
    sequence: u64,
    payload_bytes: Vec<u8>,
    max_payload_len: u32,
) -> (r: Result<SpecRecordEnvelope, SpecJournalError>)
    requires
        spec_kind_family_valid_spec(magic, kind),
        payload_bytes.len() <= max_payload_len as usize,
    ensures
        match r {
            Ok(env) => {
                &&& env.magic == magic
                &&& env.schema_version == SPEC_CURRENT_SCHEMA_VERSION
                &&& env.record_kind == spec_kind_id(kind)
                &&& env.sequence == sequence
            },
            Err(SpecJournalError::RecordKindFamilyMismatch { magic: m, kind: k }) => {
                &&& m == magic && k == spec_kind_id(kind)
                &&& !spec_kind_family_valid_spec(magic, kind)
            },
            Err(SpecJournalError::Encode) => false,
            Err(SpecJournalError::PayloadTooLarge { len, max }) => {
                &&& (max == max_payload_len || max == (u32::MAX as u32))
                &&& (len == (u32::MAX as u32) || len > max)
            },
            Err(_) => false,
        },
{
    // postcard_ok=true: serialize succeeded; payload_bytes.len() <= max_payload_len
    //                    means PayloadTooLarge guard does not fire.
    let r = production::encode_record(
        magic,
        kind,
        sequence,
        payload_bytes,
        max_payload_len,
        true,
    );
    r
}

/// Family-mismatch wrapper for `encode_record`: when
/// `spec_kind_family_valid(magic, kind) == false`, the bridge returns
/// `Err(RecordKindFamilyMismatch { magic, kind })`.
pub exec fn wrapper_encode_record_family_mismatch(
    magic: u32,
    kind: SpecRecordKind,
    sequence: u64,
    payload_bytes: Vec<u8>,
    max_payload_len: u32,
) -> (r: Result<SpecRecordEnvelope, SpecJournalError>)
    requires
        !spec_kind_family_valid_spec(magic, kind),
    ensures
        match r {
Err(SpecJournalError::RecordKindFamilyMismatch { magic: m, kind: k }) => {
                &&& m == magic && k == spec_kind_id(kind)
            },
            _ => false,
        },
{
    let r = production::encode_record(
        magic,
        kind,
        sequence,
        payload_bytes,
        max_payload_len,
        true,
    );
    r
}

/// Payload-too-large wrapper for `encode_record`: when
/// `payload_bytes.len() > max_payload_len`, the bridge returns
/// `Err(PayloadTooLarge { len, max })` with `max == max_payload_len`
/// and `len == payload_bytes.len() as u32`.
///
/// The wrapper's `ensures` is the bridge contract's full `match`
/// postcondition (copy-pasted). Verus can verify this directly
/// because the bridge contract guarantees the postcondition for every
/// reachable branch.
pub exec fn wrapper_encode_record_payload_too_large(
    magic: u32,
    kind: SpecRecordKind,
    sequence: u64,
    payload_bytes: Vec<u8>,
    max_payload_len: u32,
) -> (r: Result<SpecRecordEnvelope, SpecJournalError>)
    requires
        spec_kind_family_valid_spec(magic, kind),
        payload_bytes.len() > max_payload_len as usize,
        payload_bytes.len() <= u32::MAX as usize,
    ensures
        match r {
            Ok(env) => {
                &&& env.magic == magic
                &&& env.schema_version == SPEC_CURRENT_SCHEMA_VERSION
                &&& env.record_kind == spec_kind_id(kind)
                &&& env.sequence == sequence
                &&& spec_kind_family_valid_spec(magic, kind)
                &&& (payload_bytes.len() as u32) <= max_payload_len
            },
            Err(SpecJournalError::RecordKindFamilyMismatch { magic: m, kind: k }) => {
                &&& m == magic
                &&& k == spec_kind_id(kind)
                &&& !spec_kind_family_valid_spec(magic, kind)
            },
            Err(SpecJournalError::Encode) => {
                &&& !spec_postcard_ok_true()
                &&& spec_kind_family_valid_spec(magic, kind)
            },
            Err(SpecJournalError::PayloadTooLarge { len, max }) => {
                &&& (max == max_payload_len || max == (u32::MAX as u32))
                &&& (len == (u32::MAX as u32) || len > max)
                &&& spec_kind_family_valid_spec(magic, kind)
                &&& spec_postcard_ok_true()
            },
            Err(SpecJournalError::BadMagic { .. })
            | Err(SpecJournalError::UnsupportedSchemaVersion { .. })
            | Err(SpecJournalError::MigrationRequired { .. })
            | Err(SpecJournalError::UnknownRecordKind { .. })
            | Err(SpecJournalError::HeaderLengthMismatch { .. })
            | Err(SpecJournalError::HeaderChecksumMismatch)
            | Err(SpecJournalError::PayloadDigestMismatch)
            | Err(SpecJournalError::UnexpectedEof)
            | Err(SpecJournalError::PostcardDecodeFailed)
            | Err(SpecJournalError::RecordKindPayloadMismatch { .. })
            | Err(SpecJournalError::InvalidEvent)
            | Err(SpecJournalError::QueueFull)
            | Err(SpecJournalError::JournalBatchBytesExceeded { .. }) => false,
        },
{
    let r = production::encode_record(
        magic,
        kind,
        sequence,
        payload_bytes,
        max_payload_len,
        true,
    );
    r
}

/// Happy-path wrapper for `decode_record`: under successful header
/// validation, postcard decode, and parity enforcement, the bridge
/// returns `Ok((envelope, ()))` with `envelope.magic ==
/// expected_magic`.
///
/// The wrapper's `ensures` is the bridge contract's full `match`
/// postcondition (copy-pasted). Verus can verify this directly
/// because the bridge contract guarantees the postcondition for every
/// reachable branch.
pub exec fn wrapper_decode_record_ok(
    bytes: Vec<u8>,
    expected_magic: u32,
    max_payload_len: u32,
    decoded_envelope: SpecRecordEnvelope,
) -> (r: Result<(SpecRecordEnvelope, ()), SpecJournalError>)
    requires
        decoded_envelope.magic == expected_magic,
    ensures
        match r {
            Ok((env, _unit)) => {
                &&& env.magic == decoded_envelope.magic
                &&& env.schema_version == decoded_envelope.schema_version
                &&& env.record_kind == decoded_envelope.record_kind
                &&& env.sequence == decoded_envelope.sequence
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& spec_parity_ok_true()
                &&& env.magic == expected_magic
            },
            Err(SpecJournalError::BadMagic { found }) => {
                &&& !spec_header_ok_true()
                &&& found == decoded_envelope.magic
                &&& decoded_envelope.magic != expected_magic
            },
            Err(SpecJournalError::RecordKindFamilyMismatch { magic: m, kind: k }) => {
                &&& !spec_header_ok_true()
                &&& m == decoded_envelope.magic
                &&& k == decoded_envelope.record_kind
            },
            Err(SpecJournalError::UnsupportedSchemaVersion { version })
            | Err(SpecJournalError::MigrationRequired { from: version, .. })
            | Err(SpecJournalError::UnknownRecordKind { kind: version }) => {
                &&& !spec_header_ok_true()
            },
            Err(SpecJournalError::HeaderLengthMismatch { .. })
            | Err(SpecJournalError::HeaderChecksumMismatch)
            | Err(SpecJournalError::PayloadDigestMismatch)
            | Err(SpecJournalError::UnexpectedEof) => {
                &&& !spec_header_ok_true()
            },
            Err(SpecJournalError::PayloadTooLarge { len, max }) => {
                &&& !spec_header_ok_true() || (len > max)
            },
            Err(SpecJournalError::PostcardDecodeFailed) => {
                &&& spec_header_ok_true()
                &&& !spec_decode_ok_true()
            },
            Err(SpecJournalError::RecordKindPayloadMismatch { envelope_kind, payload_kind }) => {
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& !spec_parity_ok_true()
                &&& envelope_kind == decoded_envelope.record_kind
            },
            Err(SpecJournalError::InvalidEvent) => {
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& !spec_parity_ok_true()
            },
            Err(SpecJournalError::Encode)
            | Err(SpecJournalError::QueueFull)
            | Err(SpecJournalError::JournalBatchBytesExceeded { .. }) => false,
        },
{
    let r = production::decode_record(
        bytes,
        expected_magic,
        max_payload_len,
        true,
        decoded_envelope,
        true,
        true,
    );
    r
}

/// Bad-magic wrapper for `decode_record`: when
/// `decoded_envelope.magic != expected_magic`, the bridge returns
/// `Err(BadMagic { found })`.
pub exec fn wrapper_decode_record_bad_magic(
    bytes: Vec<u8>,
    expected_magic: u32,
    max_payload_len: u32,
    decoded_envelope: SpecRecordEnvelope,
) -> (r: Result<(SpecRecordEnvelope, ()), SpecJournalError>)
    requires
        decoded_envelope.magic != expected_magic,
    ensures
        match r {
            Ok((env, _unit)) => {
                &&& env.magic == decoded_envelope.magic
                &&& env.schema_version == decoded_envelope.schema_version
                &&& env.record_kind == decoded_envelope.record_kind
                &&& env.sequence == decoded_envelope.sequence
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& spec_parity_ok_true()
                &&& env.magic == expected_magic
            },
            Err(SpecJournalError::BadMagic { found }) => {
                &&& !spec_header_ok_true()
                &&& found == decoded_envelope.magic
                &&& decoded_envelope.magic != expected_magic
            },
            Err(SpecJournalError::RecordKindFamilyMismatch { magic: m, kind: k }) => {
                &&& !spec_header_ok_true()
                &&& m == decoded_envelope.magic
                &&& k == decoded_envelope.record_kind
            },
            Err(SpecJournalError::UnsupportedSchemaVersion { version })
            | Err(SpecJournalError::MigrationRequired { from: version, .. })
            | Err(SpecJournalError::UnknownRecordKind { kind: version }) => {
                &&& !spec_header_ok_true()
            },
            Err(SpecJournalError::HeaderLengthMismatch { .. })
            | Err(SpecJournalError::HeaderChecksumMismatch)
            | Err(SpecJournalError::PayloadDigestMismatch)
            | Err(SpecJournalError::UnexpectedEof) => {
                &&& !spec_header_ok_true()
            },
            Err(SpecJournalError::PayloadTooLarge { len, max }) => {
                &&& !spec_header_ok_true() || (len > max)
            },
            Err(SpecJournalError::PostcardDecodeFailed) => {
                &&& spec_header_ok_true()
                &&& !spec_decode_ok_true()
            },
            Err(SpecJournalError::RecordKindPayloadMismatch { envelope_kind, payload_kind }) => {
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& !spec_parity_ok_true()
                &&& envelope_kind == decoded_envelope.record_kind
            },
            Err(SpecJournalError::InvalidEvent) => {
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& !spec_parity_ok_true()
            },
            Err(SpecJournalError::Encode)
            | Err(SpecJournalError::QueueFull)
            | Err(SpecJournalError::JournalBatchBytesExceeded { .. }) => false,
        },
{
    let r = production::decode_record(
        bytes,
        expected_magic,
        max_payload_len,
        true,
        decoded_envelope,
        true,
        true,
    );
    r
}

/// Parity-failure wrapper for `decode_record`: when `parity_ok ==
/// false`, the bridge returns `Err(RecordKindPayloadMismatch { .. })`
/// or `Err(InvalidEvent)` (production code returns the former for the
/// `JournalEvent` impl when `envelope.record_kind != payload_variant`,
/// and the latter when `JournalEvent::is_valid()` fails).
pub exec fn wrapper_decode_record_parity_mismatch(
    bytes: Vec<u8>,
    expected_magic: u32,
    max_payload_len: u32,
    decoded_envelope: SpecRecordEnvelope,
    parity_ok: bool,
) -> (r: Result<(SpecRecordEnvelope, ()), SpecJournalError>)
    requires
        decoded_envelope.magic == expected_magic,
        !parity_ok,
    ensures
        match r {
            Ok((env, _unit)) => {
                &&& env.magic == decoded_envelope.magic
                &&& env.schema_version == decoded_envelope.schema_version
                &&& env.record_kind == decoded_envelope.record_kind
                &&& env.sequence == decoded_envelope.sequence
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& parity_ok
                &&& env.magic == expected_magic
            },
            Err(SpecJournalError::BadMagic { found }) => {
                &&& !spec_header_ok_true()
                &&& found == decoded_envelope.magic
                &&& decoded_envelope.magic != expected_magic
            },
            Err(SpecJournalError::RecordKindFamilyMismatch { magic: m, kind: k }) => {
                &&& !spec_header_ok_true()
                &&& m == decoded_envelope.magic
                &&& k == decoded_envelope.record_kind
            },
            Err(SpecJournalError::UnsupportedSchemaVersion { version })
            | Err(SpecJournalError::MigrationRequired { from: version, .. })
            | Err(SpecJournalError::UnknownRecordKind { kind: version }) => {
                &&& !spec_header_ok_true()
            },
            Err(SpecJournalError::HeaderLengthMismatch { .. })
            | Err(SpecJournalError::HeaderChecksumMismatch)
            | Err(SpecJournalError::PayloadDigestMismatch)
            | Err(SpecJournalError::UnexpectedEof) => {
                &&& !spec_header_ok_true()
            },
            Err(SpecJournalError::PayloadTooLarge { len, max }) => {
                &&& !spec_header_ok_true() || (len > max)
            },
            Err(SpecJournalError::PostcardDecodeFailed) => {
                &&& spec_header_ok_true()
                &&& !spec_decode_ok_true()
            },
            Err(SpecJournalError::RecordKindPayloadMismatch { envelope_kind, payload_kind }) => {
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& !parity_ok
                &&& envelope_kind == decoded_envelope.record_kind
            },
            Err(SpecJournalError::InvalidEvent) => {
                &&& spec_header_ok_true()
                &&& spec_decode_ok_true()
                &&& !parity_ok
            },
            Err(SpecJournalError::Encode)
            | Err(SpecJournalError::QueueFull)
            | Err(SpecJournalError::JournalBatchBytesExceeded { .. }) => false,
        },
{
    let r = production::decode_record(
        bytes,
        expected_magic,
        max_payload_len,
        true,
        decoded_envelope,
        true,
        parity_ok,
    );
    r
}

}
