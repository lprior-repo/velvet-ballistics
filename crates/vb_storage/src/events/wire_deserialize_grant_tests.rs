#![forbid(unsafe_code)]
//! Source-derived historical wire fixtures, not old-binary producer evidence.
//! Counters measure owned materializer calls, not setup allocations or RSS.

use super::{decode_with_materializer, materialize_owned};
use crate::codec::{decode_journal_event, decode_record, encode_record_header};
use crate::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use crate::{EventSeq, JournalError, JournalEvent, RecordKind};
use std::cell::Cell;
use vb_core::capability::Capability;
use vb_core::capability_grant::{
    CapabilityGrant, GrantId, GrantIdentity, HistoricalGrantSignature, ResourceScope,
    RevocationEpoch,
};
use vb_core::manual_gate::GateDigest;
use vb_core::{ActionId, RunId, WorkflowDigest};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn grant() -> CapabilityGrant {
    CapabilityGrant {
        id: GrantId::new(1),
        issuer: GrantIdentity {
            value: "issuer".into(),
        },
        subject: GrantIdentity {
            value: "subject".into(),
        },
        artifact_digest: WorkflowDigest::from_bytes([7; 32]),
        run: RunId::new(3),
        action: ActionId::new(9),
        capabilities: Box::new([Capability::new("read".into(), ActionId::new(9))]),
        resources: ResourceScope {
            patterns: Box::new(["repo-a".into()]),
        },
        issued_at_ms: 100,
        expires_at_ms: 1_000,
        delegable: false,
        revocation_epoch: RevocationEpoch::new(2),
        break_glass: false,
        approval_receipt_digest: None,
        signature: None,
    }
}

fn payload(grant: &CapabilityGrant) -> TestResult<Vec<u8>> {
    // A tuple fixture can encode malformed data independently of event validation.
    Ok(postcard::to_allocvec(&(
        46_u16,
        RunId::new(3),
        EventSeq::new(5),
        grant,
    ))?)
}

fn record(kind: RecordKind, sequence: u64, payload: &[u8]) -> TestResult<Vec<u8>> {
    record_with_limit(kind, sequence, payload, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
}

fn record_with_limit(
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    limit: u32,
) -> TestResult<Vec<u8>> {
    let mut bytes =
        encode_record_header(MAGIC_JOURNAL_EVENT, kind, sequence, payload, limit)?.to_vec();
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

fn assert_rejected(bytes: &[u8], label: &str) -> TestResult {
    let encoded = record(RecordKind::CapabilityGrant, 5, bytes)?;
    // CRC and payload digest must pass before any semantic refusal is counted.
    let (envelope, checked) = crate::codec::payload::decode_record_payload(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let calls = Cell::new(0);
    let result = decode_with_materializer(&envelope, checked, |raw| {
        calls.set(1);
        materialize_owned(raw)
    });
    assert!(
        result.is_err(),
        "{label}: private guard accepted invalid data"
    );
    assert_eq!(calls.get(), 0, "{label}: owned materializer was invoked");
    assert!(
        decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .is_err(),
        "{label}: public contextual codec accepted invalid data"
    );
    assert!(
        decode_journal_event(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES
        )
        .is_err(),
        "{label}: strong journal decoder accepted invalid data"
    );
    Ok(())
}

fn assert_accepted(grant: CapabilityGrant) -> TestResult {
    assert_accepted_with_limit(grant, MAX_JOURNAL_EVENT_PAYLOAD_BYTES).map(|_| ())
}

fn assert_accepted_with_limit(grant: CapabilityGrant, limit: u32) -> TestResult<Vec<u8>> {
    assert!(grant.is_valid());
    let raw = payload(&grant)?;
    let expected = JournalEvent::CapabilityGrantEvent {
        run: RunId::new(3),
        seq: EventSeq::new(5),
        grant,
    };
    assert!(expected.is_valid());
    assert_eq!(postcard::to_allocvec(&expected)?, raw);
    let encoded = record_with_limit(RecordKind::CapabilityGrant, 5, &raw, limit)?;
    let (envelope, checked) =
        crate::codec::payload::decode_record_payload(&encoded, MAGIC_JOURNAL_EVENT, limit)?;
    let calls = Cell::new(0);
    let (decoded, remaining) = decode_with_materializer(&envelope, checked, |bytes| {
        calls.set(1);
        materialize_owned(bytes)
    })?;
    assert_eq!(calls.get(), 1);
    assert!(remaining.is_empty());
    assert_eq!(decoded, expected);
    assert_eq!(
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, limit,)?.1,
        expected
    );
    assert_eq!(
        decode_journal_event(&encoded, MAGIC_JOURNAL_EVENT, limit,)?.1,
        expected
    );
    assert!(postcard::from_bytes::<JournalEvent>(&raw).is_err());
    Ok(encoded)
}

#[test]
fn manual_historical_tag46_bytes_require_context() -> TestResult {
    // Independently enumerated fifteen-field Postcard layout, including tag/run/seq.
    const GOLDEN: &[u8] = &[
        46, 3, 5, 1, 6, 105, 115, 115, 117, 101, 114, 7, 115, 117, 98, 106, 101, 99, 116, 7, 7, 7,
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 3,
        9, 1, 4, 114, 101, 97, 100, 9, 1, 6, 114, 101, 112, 111, 45, 97, 100, 232, 7, 0, 2, 0, 0,
        0,
    ];
    assert_eq!(payload(&grant())?, GOLDEN);
    assert_accepted(grant())
}

#[test]
fn exact_limits_and_all_receipt_signature_combinations_remain_valid() -> TestResult {
    for (receipt, signature_length) in [
        (false, None),
        (true, None),
        (false, Some(128)),
        (true, Some(1024)),
    ] {
        let mut value = grant();
        value.issuer.value = "é".repeat(64);
        value.subject.value = "s".repeat(128);
        value.capabilities =
            vec![Capability::new("read".into(), ActionId::new(9)); 256].into_boxed_slice();
        // Nonempty capability names historically have no 128-byte ceiling.
        *value
            .capabilities
            .last_mut()
            .ok_or("missing last capability")? =
            Capability::new("é".repeat(65).into_boxed_str(), ActionId::new(9));
        value.resources.patterns = vec!["é".repeat(64).into_boxed_str(); 256].into_boxed_slice();
        value.approval_receipt_digest = receipt.then(|| GateDigest::from_bytes([31; 32]));
        value.signature = signature_length.map(|length| HistoricalGrantSignature {
            bytes: vec![0xa1; length].into_boxed_slice(),
        });
        assert_accepted(value)?;
    }
    let mut empty_resources = grant();
    empty_resources.resources.patterns = Box::new([]);
    empty_resources.signature = Some(HistoricalGrantSignature {
        bytes: Box::new([1]),
    });
    assert_accepted(empty_resources)
}

#[test]
fn explicit_larger_payload_limit_accepts_historical_long_capability_name() -> TestResult {
    let larger_limit = MAX_JOURNAL_EVENT_PAYLOAD_BYTES
        .checked_mul(2)
        .ok_or("limit overflow")?;
    let name_length = usize::try_from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?
        .checked_add(1)
        .ok_or("name length overflow")?;
    let mut value = grant();
    value.capabilities = Box::new([Capability::new(
        "a".repeat(name_length).into_boxed_str(),
        ActionId::new(9),
    )]);
    // The same checksummed bytes must honor the caller's explicit payload limit.
    let encoded = assert_accepted_with_limit(value, larger_limit)?;
    for result in [
        decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ),
        decode_journal_event(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ),
    ] {
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { len, max })
            if len > max && max == MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
        );
    }
    Ok(())
}

#[test]
fn invalid_owned_shapes_fail_before_owned_materialization() -> TestResult {
    type Mutation = (&'static str, fn(&mut CapabilityGrant));
    let cases: &[Mutation] = &[
        ("empty issuer", |g| g.issuer.value.clear()),
        ("issuer UTF8 byte bound", |g| {
            g.issuer.value = "é".repeat(65)
        }),
        ("empty subject", |g| g.subject.value.clear()),
        ("subject byte bound", |g| g.subject.value = "s".repeat(129)),
        ("empty capabilities", |g| g.capabilities = Box::new([])),
        ("257 capabilities", |g| {
            g.capabilities =
                vec![Capability::new("read".into(), ActionId::new(9)); 257].into_boxed_slice()
        }),
        ("last capability invalid", |g| {
            g.capabilities = Box::new([
                Capability::new("read".into(), ActionId::new(9)),
                Capability::new("".into(), ActionId::new(9)),
            ])
        }),
        ("257 resources", |g| {
            g.resources.patterns = vec!["repo-a".into(); 257].into_boxed_slice()
        }),
        ("last resource empty", |g| {
            g.resources.patterns = Box::new(["repo-a".into(), "".into()])
        }),
        ("last resource UTF8 byte bound", |g| {
            g.resources.patterns = Box::new(["repo-a".into(), "é".repeat(65).into_boxed_str()])
        }),
        ("empty signature", |g| {
            g.signature = Some(HistoricalGrantSignature {
                bytes: Box::new([]),
            })
        }),
        ("1025 signature bytes", |g| {
            g.signature = Some(HistoricalGrantSignature {
                bytes: vec![1; 1025].into_boxed_slice(),
            })
        }),
        ("equal times", |g| g.expires_at_ms = g.issued_at_ms),
        ("reversed times", |g| g.expires_at_ms = g.issued_at_ms - 1),
    ];
    for (label, change) in cases {
        let mut value = grant();
        assert!(value.is_valid(), "{label}: invalid positive control");
        change(&mut value);
        assert!(!value.is_valid(), "{label}: invalidity not isolated");
        let raw = payload(&value)?;
        let event = JournalEvent::CapabilityGrantEvent {
            run: RunId::new(3),
            seq: EventSeq::new(5),
            grant: value,
        };
        assert!(!event.is_valid(), "{label}: invalid event accepted");
        assert!(
            postcard::to_allocvec(&event).is_err(),
            "{label}: invalid event serialized"
        );
        assert_rejected(&raw, label)?;
    }
    let raw = payload(&grant())?;
    for (label, outer_run, seq, inner_run) in [
        ("zero outer run", 0, 5, 0),
        ("exhausted sequence", 3, u64::MAX, 3),
        ("run mismatch", 3, 5, 4),
    ] {
        let mut value = grant();
        value.run = RunId::new(inner_run);
        let malformed =
            postcard::to_allocvec(&(46_u16, RunId::new(outer_run), EventSeq::new(seq), value))?;
        assert_rejected(&malformed, label)?;
    }
    assert!(!raw.is_empty());
    Ok(())
}

#[test]
fn malformed_utf8_scalars_truncations_and_trailing_bytes_never_materialize() -> TestResult {
    let raw = payload(&grant())?;
    for end in 0..raw.len() {
        assert_rejected(
            raw.get(..end).ok_or("truncation outside fixture")?,
            "truncated field",
        )?;
    }
    for needle in [b"issuer".as_slice(), b"subject", b"read", b"repo-a"] {
        let mut malformed = raw.clone();
        let offset = malformed
            .windows(needle.len())
            .position(|window| window == needle)
            .ok_or("missing text marker")?;
        *malformed.get_mut(offset).ok_or("invalid text offset")? = 0xff;
        assert_rejected(&malformed, "invalid UTF8")?;
    }
    // This None/None fixture has one-byte epoch 2, bools and option tags.
    for distance in [5, 3, 2, 1] {
        let mut malformed = raw.clone();
        let offset = malformed
            .len()
            .checked_sub(distance)
            .ok_or("missing scalar")?;
        *malformed.get_mut(offset).ok_or("invalid scalar offset")? = 2;
        assert_rejected(&malformed, "invalid bool/option discriminant")?;
    }
    let mut trailing = raw;
    trailing.push(0);
    assert_rejected(&trailing, "trailing data")
}

#[test]
fn oversized_advertised_collections_and_short_bodies_never_materialize() -> TestResult {
    let value = grant();
    // Serialize only the prefix: attacker-advertised counts have no owned bodies.
    let before_capabilities = postcard::to_allocvec(&(
        46_u16,
        RunId::new(3),
        EventSeq::new(5),
        value.id,
        &value.issuer,
        &value.subject,
        value.artifact_digest,
        value.run,
        value.action,
    ))?;
    let before_resources = postcard::to_allocvec(&(
        46_u16,
        RunId::new(3),
        EventSeq::new(5),
        value.id,
        &value.issuer,
        &value.subject,
        value.artifact_digest,
        value.run,
        value.action,
        &value.capabilities,
    ))?;
    let mut before_signature = payload(&value)?;
    before_signature.pop().ok_or("missing signature option")?;
    before_signature.push(1); // signature Some
    for (prefix, limits) in [
        (before_capabilities, [257_u64, u64::MAX]),
        (before_resources, [257_u64, u64::MAX]),
        (before_signature, [1025_u64, u64::MAX]),
    ] {
        for count in limits {
            let mut malformed = prefix.clone();
            malformed.extend_from_slice(&postcard::to_allocvec(&count)?);
            assert_rejected(&malformed, "advertised oversize with absent body")?;
        }
    }
    Ok(())
}

#[test]
fn tag46_never_uses_generic_serde_or_legacy_fallback() -> TestResult {
    let accepted = JournalEvent::RunAccepted {
        run: RunId::new(3),
        seq: EventSeq::new(5),
        workflow: WorkflowDigest::from_bytes([7; 32]),
    };
    let stable = postcard::to_allocvec(&accepted)?;
    let legacy = postcard::to_allocvec(&(
        0_u16,
        RunId::new(3),
        EventSeq::new(5),
        WorkflowDigest::from_bytes([7; 32]),
    ))?;
    for (label, raw) in [
        ("valid stable other kind", stable),
        ("valid legacy ordinal", legacy),
    ] {
        let control = record(RecordKind::RunAccepted, 5, &raw)?;
        assert_eq!(
            decode_record::<JournalEvent>(
                &control,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES
            )?
            .1,
            accepted
        );
        assert_rejected(&raw, label)?;
        let wrong = record(RecordKind::CapabilityGrant, 5, &raw)?;
        assert!(matches!(
            decode_record::<JournalEvent>(
                &wrong,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES
            ),
            Err(JournalError::RecordKindPayloadMismatch {
                envelope_kind: 46,
                ..
            })
        ));
    }
    let raw = payload(&grant())?;
    assert!(postcard::take_from_bytes::<JournalEvent>(&raw).is_err());
    let wrong = record(RecordKind::RunAccepted, 5, &raw)?;
    let (envelope, checked) = crate::codec::payload::decode_record_payload(
        &wrong,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let calls = Cell::new(0);
    assert!(
        decode_with_materializer(&envelope, checked, |bytes| {
            calls.set(1);
            materialize_owned(bytes)
        })
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert!(matches!(
        decode_record::<JournalEvent>(&wrong, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        Err(JournalError::RecordKindPayloadMismatch {
            payload_kind: 46,
            ..
        })
    ));
    Ok(())
}

#[test]
fn envelope_sequence_check_stays_specific_to_journal_decode() -> TestResult {
    let raw = payload(&grant())?;
    let bytes = record(RecordKind::CapabilityGrant, 6, &raw)?;
    let (envelope, checked) = crate::codec::payload::decode_record_payload(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let calls = Cell::new(0);
    let (_, remaining) = decode_with_materializer(&envelope, checked, |payload| {
        calls.set(1);
        materialize_owned(payload)
    })?;
    assert!(remaining.is_empty());
    assert_eq!(calls.get(), 1);
    assert!(
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
            .is_ok()
    );
    assert!(matches!(
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        Err(JournalError::ReplayEnvelopeSequenceMismatch {
            envelope_seq: 6,
            payload_seq: 5,
            ..
        })
    ));
    Ok(())
}
